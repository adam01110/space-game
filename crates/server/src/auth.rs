use std::{
    io::Write,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use bevy::prelude::Resource;
use lightyear::netcode::generate_key;
use project_protocol::{
    security::{encode_hex, GuestCredentials},
    SERVER_PORT,
};
use tiny_http::{Method, Request, Server};

use crate::security::{issue_token, ServerKey};

const LISTEN_ADDRESS: &str = "127.0.0.1:5001";
const RATE_LIMIT: u32 = 20;
const RATE_WINDOW: Duration = Duration::from_secs(1);
const RECEIVE_TIMEOUT: Duration = Duration::from_millis(100);

type StartError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Resource)]
pub(super) struct AuthService {
    stop: Arc<AtomicBool>,
    _worker: JoinHandle<()>,
}

impl AuthService {
    pub(super) fn start(key: [u8; 32], certificate_digest: String) -> Result<Self, StartError> {
        let public_address = public_address()?;
        // tiny_http binds before it returns, so startup reports a listener failure synchronously.
        let server = Server::http(LISTEN_ADDRESS)?;
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = std::thread::Builder::new()
            .name("guest-auth".to_owned())
            .spawn(move || run(server, worker_stop, key, public_address, certificate_digest))?;

        Ok(Self {
            stop,
            _worker: worker,
        })
    }
}

impl Drop for AuthService {
    fn drop(&mut self) {
        // The handle is deliberately detached on drop: waiting on an uncooperative HTTP peer
        // could block Bevy shutdown. The receive timeout lets an idle worker exit promptly.
        self.stop.store(true, Ordering::Release);
    }
}

fn public_address() -> Result<SocketAddr, StartError> {
    let configured = std::env::var("PROJECT_SERVER_ADDRESS");
    #[cfg(feature = "dev")]
    let address = match configured {
        Ok(address) => address.parse()?,
        Err(std::env::VarError::NotPresent) => SocketAddr::from(([127, 0, 0, 1], SERVER_PORT)),
        Err(error) => return Err(error.into()),
    };
    #[cfg(not(feature = "dev"))]
    let address: SocketAddr = configured
        .map_err(|_| "PROJECT_SERVER_ADDRESS must be set to a reachable IPv4 address on port 5000")?
        .parse()?;

    if !address.is_ipv4() || address.ip().is_unspecified() || address.port() != SERVER_PORT {
        return Err("PROJECT_SERVER_ADDRESS must be a reachable IPv4 address on port 5000".into());
    }
    Ok(address)
}

fn run(
    server: Server,
    stop: Arc<AtomicBool>,
    key: [u8; 32],
    public_address: SocketAddr,
    certificate_digest: String,
) {
    let mut limiter = RateLimiter::new(Instant::now());
    while !stop.load(Ordering::Acquire) {
        match server.recv_timeout(RECEIVE_TIMEOUT) {
            Ok(Some(request)) => handle_request(
                request,
                &mut limiter,
                &key,
                public_address,
                &certificate_digest,
            ),
            Ok(None) => {}
            Err(_) => break,
        }
    }
}

fn handle_request(
    request: Request,
    limiter: &mut RateLimiter,
    key: &[u8; 32],
    public_address: SocketAddr,
    certificate_digest: &str,
) {
    match validate_request(request.method(), request.url(), request_has_body(&request)) {
        RequestValidation::WrongMethod => {
            send_response(
                request,
                405,
                "Method Not Allowed",
                b"{\"error\":\"method not allowed\"}",
                None,
            );
        }
        RequestValidation::WrongRoute => {
            send_response(
                request,
                404,
                "Not Found",
                b"{\"error\":\"not found\"}",
                None,
            );
        }
        RequestValidation::HasBody => {
            send_response(
                request,
                413,
                "Content Too Large",
                b"{\"error\":\"request body not allowed\"}",
                None,
            );
        }
        RequestValidation::Valid if !limiter.allow(Instant::now()) => {
            send_response(
                request,
                429,
                "Too Many Requests",
                b"{\"error\":\"rate limit exceeded\"}",
                Some(1),
            );
        }
        RequestValidation::Valid => match credentials_json(key, public_address, certificate_digest)
        {
            Ok(body) => send_response(request, 200, "OK", &body, None),
            Err(_) => send_response(
                request,
                500,
                "Internal Server Error",
                b"{\"error\":\"credential issuance failed\"}",
                None,
            ),
        },
    }
}

fn request_has_body(request: &Request) -> bool {
    if request
        .headers()
        .iter()
        .any(|header| header.field.equiv("Transfer-Encoding"))
    {
        return true;
    }

    let mut lengths = request
        .headers()
        .iter()
        .filter(|header| header.field.equiv("Content-Length"));
    lengths
        .next()
        .is_some_and(|header| header.value.as_str() != "0" || lengths.next().is_some())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RequestValidation {
    Valid,
    WrongMethod,
    WrongRoute,
    HasBody,
}

fn validate_request(method: &Method, route: &str, has_body: bool) -> RequestValidation {
    if method != &Method::Post {
        RequestValidation::WrongMethod
    } else if route != "/connect" {
        RequestValidation::WrongRoute
    } else if has_body {
        RequestValidation::HasBody
    } else {
        RequestValidation::Valid
    }
}

fn guest_id() -> u64 {
    let [byte_0, byte_1, byte_2, byte_3, byte_4, byte_5, byte_6, byte_7, ..] = generate_key();
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

fn send_response(
    request: Request,
    status: u16,
    reason: &str,
    body: &[u8],
    retry_after_seconds: Option<u64>,
) {
    // Response::with_header intentionally discards Connection headers in tiny_http 0.12.
    // Writing the small, fully framed response directly is the supported escape hatch needed
    // to tell browsers and the loopback reverse proxy not to keep this connection alive.
    let mut writer = request.into_writer();
    if write!(
        writer,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    )
    .is_err()
    {
        return;
    }
    let retry_header_failed = retry_after_seconds
        .is_some_and(|seconds| write!(writer, "Retry-After: {seconds}\r\n").is_err());
    if retry_header_failed {
        return;
    }
    if writer.write_all(b"\r\n").is_err() {
        return;
    }
    if writer.write_all(body).is_err() {
        return;
    }
    drop(writer.flush());
}

struct RateLimiter {
    window_started: Instant,
    issued: u32,
}

impl RateLimiter {
    const fn new(now: Instant) -> Self {
        Self {
            window_started: now,
            issued: 0,
        }
    }

    fn allow(&mut self, now: Instant) -> bool {
        if now
            .checked_duration_since(self.window_started)
            .is_some_and(|elapsed| elapsed >= RATE_WINDOW)
        {
            self.window_started = now;
            self.issued = 0;
        }
        if self.issued >= RATE_LIMIT {
            return false;
        }
        self.issued += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightyear::netcode::{ConnectToken, CONNECT_TOKEN_BYTES};
    use project_protocol::security::decode_hex;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn limiter_allows_twenty_requests_per_window() {
        let start = Instant::now();
        let mut limiter = RateLimiter::new(start);
        for _ in 0..RATE_LIMIT {
            assert!(limiter.allow(start));
        }
        assert!(!limiter.allow(start));
        assert!(limiter.allow(start + RATE_WINDOW));
    }

    #[test]
    fn validates_method_route_and_body() {
        assert_eq!(
            validate_request(&Method::Post, "/connect", false),
            RequestValidation::Valid
        );
        assert_eq!(
            validate_request(&Method::Get, "/connect", false),
            RequestValidation::WrongMethod
        );
        assert_eq!(
            validate_request(&Method::Post, "/connect?x=1", false),
            RequestValidation::WrongRoute
        );
        assert_eq!(
            validate_request(&Method::Post, "/connect", true),
            RequestValidation::HasBody
        );
    }

    #[test]
    fn token_response_round_trips_and_expires_after_sixty_seconds() {
        let now = || {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_secs()
        };
        let before = now();
        let body = credentials_json(
            &generate_key(),
            SocketAddr::from(([127, 0, 0, 1], SERVER_PORT)),
            "sha-256=test",
        )
        .expect("credentials");
        let credentials: GuestCredentials = serde_json::from_slice(&body).expect("JSON response");
        assert_eq!(credentials.certificate_digest, "sha-256=test");
        let bytes =
            decode_hex::<CONNECT_TOKEN_BYTES>(&credentials.connect_token).expect("token hex");
        let token = ConnectToken::try_from_bytes(&bytes).expect("connect token");
        assert!((before + 60..=now() + 60).contains(&token.expire_timestamp()));
    }
}
