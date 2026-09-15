use std::{net::SocketAddr, num::NonZeroU32, sync::Arc, time::Duration};

use bevy::prelude::Resource;
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use lightyear::netcode::generate_key;
use project_protocol::{
    SERVER_PORT,
    security::{GuestCredentials, encode_hex},
};
use tii::{
    MimeType, RequestContext, Response, ServerBuilder, StatusCode, TiiResult,
    extras::{Connector, TcpConnector},
};

use super::security::{ServerKey, issue_token};

const LISTEN_ADDRESS: &str = "127.0.0.1:5001";
const RATE_LIMIT: u32 = 20;
const TOO_MANY_REQUESTS: StatusCode = StatusCode::from_custom(429, "Too Many Requests");

type StartError = String;

#[derive(Resource)]
pub(super) struct AuthService {
    connector: TcpConnector,
}

impl AuthService {
    pub(super) fn start(key: [u8; 32], certificate_digest: String) -> Result<Self, StartError> {
        let quota = Quota::per_second(
            NonZeroU32::new(RATE_LIMIT).ok_or("the guest rate limit must be nonzero")?,
        );

        let state = Arc::new(AuthState {
            key,
            public_address: public_address()?,
            certificate_digest,
            limiter: RateLimiter::direct(quota),
        });

        let server = ServerBuilder::builder_arc(move |builder| {
            builder
                .router(|router| router.route_post("/connect", (state, connect)))?
                .with_not_found_handler(not_found)?
                .with_keep_alive_timeout(Some(Duration::ZERO))
        })
        .map_err(|error| format!("cannot configure the guest endpoint: {error}"))?;

        let connector = TcpConnector::start_unpooled(LISTEN_ADDRESS, server)
            .map_err(|error| format!("cannot start the guest endpoint: {error}"))?;
        Ok(Self { connector })
    }
}

impl Drop for AuthService {
    fn drop(&mut self) {
        self.connector.shutdown();
    }
}

struct AuthState {
    key: [u8; 32],
    public_address: SocketAddr,
    certificate_digest: String,
    limiter: DefaultDirectRateLimiter,
}

fn parse_address(value: &str) -> Result<SocketAddr, StartError> {
    value
        .parse()
        .map_err(|error| format!("invalid PROJECT_SERVER_ADDRESS: {error}"))
}

#[cfg(feature = "dev")]
fn configured_public_address() -> Result<SocketAddr, StartError> {
    match std::env::var("PROJECT_SERVER_ADDRESS") {
        Ok(address) => parse_address(&address),
        Err(std::env::VarError::NotPresent) => Ok(SocketAddr::from(([127, 0, 0, 1], SERVER_PORT))),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(not(feature = "dev"))]
fn configured_public_address() -> Result<SocketAddr, StartError> {
    let address = std::env::var("PROJECT_SERVER_ADDRESS").map_err(|error| {
        format!(
            "PROJECT_SERVER_ADDRESS must be set to a reachable IPv4 address on port 5000: {error}"
        )
    })?;
    parse_address(&address)
}

fn validate_public_address(address: SocketAddr) -> Result<SocketAddr, StartError> {
    if address.is_ipv4() && !address.ip().is_unspecified() && address.port() == SERVER_PORT {
        Ok(address)
    } else {
        Err("PROJECT_SERVER_ADDRESS must be a reachable IPv4 address on port 5000".to_owned())
    }
}

fn public_address() -> Result<SocketAddr, StartError> {
    validate_public_address(configured_public_address()?)
}

fn request_error(request: &RequestContext) -> Option<(StatusCode, &'static [u8])> {
    if !request.get_query().is_empty() {
        Some((StatusCode::NotFound, b"{\"error\":\"not found\"}"))
    } else if request_has_body(request) {
        Some((
            StatusCode::ContentTooLarge,
            b"{\"error\":\"request body not allowed\"}",
        ))
    } else {
        None
    }
}

fn credentials_response(state: &AuthState) -> TiiResult<Response> {
    match credentials_json(&state.key, state.public_address, &state.certificate_digest) {
        Ok(body) => response(StatusCode::OK, body),
        Err(error) => {
            bevy::log::error!("cannot issue guest credentials: {error}");
            response(
                StatusCode::InternalServerError,
                b"{\"error\":\"credential issuance failed\"}",
            )
        }
    }
}

fn connect(state: &AuthState, request: &RequestContext) -> TiiResult<Response> {
    if let Some((status, body)) = request_error(request) {
        return response(status, body);
    }
    if state.limiter.check().is_err() {
        return response(TOO_MANY_REQUESTS, b"{\"error\":\"rate limit exceeded\"}")?
            .with_header("Retry-After", "1");
    }
    credentials_response(state)
}

fn not_found(request: &mut RequestContext) -> TiiResult<Response> {
    if request.get_path() == "/connect" {
        response(
            StatusCode::MethodNotAllowed,
            b"{\"error\":\"method not allowed\"}",
        )
    } else {
        response(StatusCode::NotFound, b"{\"error\":\"not found\"}")
    }
}

fn request_has_body(request: &RequestContext) -> bool {
    let lengths = request.get_headers("Content-Length");
    request.get_header("Transfer-Encoding").is_some() || !matches!(lengths.as_slice(), [] | ["0"])
}

fn response(status: StatusCode, body: impl AsRef<[u8]>) -> TiiResult<Response> {
    Response::new(status)
        .with_body_slice(body)
        .with_header("Content-Type", MimeType::ApplicationJson)?
        .with_header("Cache-Control", "no-store")
}

fn guest_id() -> u64 {
    let [
        byte_0,
        byte_1,
        byte_2,
        byte_3,
        byte_4,
        byte_5,
        byte_6,
        byte_7,
        ..,
    ] = generate_key();
    u64::from_le_bytes([
        byte_0, byte_1, byte_2, byte_3, byte_4, byte_5, byte_6, byte_7,
    ])
}

fn credentials_json(
    key: &[u8; 32],
    public_address: SocketAddr,
    certificate_digest: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let token = issue_token(&ServerKey(*key), public_address, guest_id())?;
    let credentials = GuestCredentials {
        connect_token: encode_hex(&token.try_into_bytes()?),
        certificate_digest: certificate_digest.to_owned(),
    };

    Ok(serde_json::to_vec(&credentials)?)
}
