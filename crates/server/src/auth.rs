mod credentials;
mod http;

use std::{net::SocketAddr, num::NonZeroU32, sync::Arc, time::Duration};

use bevy::prelude::Resource;
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use http::{connect, not_found};
use tii::{
    ServerBuilder,
    extras::{Connector, TcpConnector},
};

use project_protocol::SERVER_PORT;

const LISTEN_ADDRESS: &str = "127.0.0.1:5001";
const RATE_LIMIT: u32 = 20;

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

pub(super) struct AuthState {
    pub(super) key: [u8; 32],
    pub(super) public_address: SocketAddr,
    pub(super) certificate_digest: String,
    pub(super) limiter: DefaultDirectRateLimiter,
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
