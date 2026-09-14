//! Fetch short-lived credentials without blocking the game loop.
use std::time::Duration;

use crossbeam_channel::Receiver;
use lightyear::netcode::{ConnectToken, CONNECT_TOKEN_BYTES};
use project_protocol::security::{decode_hex, GuestCredentials};
use url::{Host, Url};

pub(super) struct Credentials {
    pub token: ConnectToken,
    pub certificate_digest: String,
}

pub(super) type GuestResult = Result<Credentials, String>;

fn validate_endpoint(value: &str, allow_loopback_http: bool) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_error| "Invalid guest endpoint URL".to_owned())?;
    let loopback = match url.host() {
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        Some(Host::Domain("localhost")) => true,
        _ => false,
    };
    if url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !(url.scheme() == "https" || (allow_loopback_http && url.scheme() == "http" && loopback))
    {
        return Err("Guest endpoint requires HTTPS (HTTP loopback is development-only)".to_owned());
    }
    Ok(url)
}

fn configured_endpoint() -> Result<Url, String> {
    #[cfg(not(target_family = "wasm"))]
    if let Some(value) = std::env::var_os("PROJECT_AUTH_URL") {
        let value = value
            .into_string()
            .map_err(|_value| "PROJECT_AUTH_URL must be Unicode".to_owned())?;
        return validate_endpoint(&value, cfg!(feature = "dev"));
    }
    if let Some(value) = option_env!("PROJECT_AUTH_URL") {
        return validate_endpoint(value, cfg!(feature = "dev"));
    }
    #[cfg(target_family = "wasm")]
    {
        let origin = web_sys::window()
            .ok_or_else(|| "Browser window is unavailable".to_owned())?
            .location()
            .origin()
            .map_err(|_error| "Cannot determine the page origin".to_owned())?;
        validate_endpoint(&format!("{origin}/connect"), cfg!(feature = "dev"))
    }
    #[cfg(not(target_family = "wasm"))]
    {
        if cfg!(feature = "dev") {
            validate_endpoint("http://127.0.0.1:5001/connect", true)
        } else {
            Err("This build has no guest endpoint configured".to_owned())
        }
    }
}

fn credentials(response: &ehttp::Response, endpoint: &Url) -> GuestResult {
    // Never trust credentials obtained after a redirect, including HTTPS downgrades.
    if response.url != endpoint.as_str() {
        return Err("Guest endpoint redirected; connection refused".to_owned());
    }
    if response.status != 200 {
        return Err(format!("Guest endpoint returned HTTP {}", response.status));
    }
    if response.bytes.len() > 8192 {
        return Err("Guest response is too large".to_owned());
    }
    let response: GuestCredentials = serde_json::from_slice(&response.bytes)
        .map_err(|_error| "Invalid guest response".to_owned())?;
    decode_hex::<32>(&response.certificate_digest)
        .map_err(|_error| "Invalid server certificate fingerprint".to_owned())?;
    let bytes = decode_hex::<CONNECT_TOKEN_BYTES>(&response.connect_token)
        .map_err(|_error| "Invalid connection token encoding".to_owned())?;
    let token = ConnectToken::try_from_bytes(&bytes)
        .map_err(|_error| "Invalid connection token".to_owned())?;
    Ok(Credentials {
        token,
        certificate_digest: response.certificate_digest,
    })
}

pub(super) fn request() -> Result<Receiver<GuestResult>, String> {
    let endpoint = configured_endpoint()?;
    let mut request = ehttp::Request::post(endpoint.as_str(), Vec::new());
    request.timeout = Some(Duration::from_secs(10));
    request.headers.insert("Accept", "application/json");
    let (sender, receiver) = crossbeam_channel::bounded(1);
    ehttp::fetch(request, move |response| {
        let result = response
            .map_err(|_error| {
                "Cannot reach the guest endpoint or verify its HTTPS certificate".to_owned()
            })
            .and_then(|response| credentials(&response, &endpoint));
        if let Err(_closed) = sender.send(result) {
            bevy::log::debug!("Guest response discarded after connection attempt ended");
        }
    });
    Ok(receiver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightyear::netcode::generate_key;
    use project_protocol::{security::encode_hex, PROTOCOL_ID, SERVER_PORT};
    use std::net::SocketAddr;

    #[test]
    fn production_requires_https() {
        assert!(validate_endpoint("https://game.example/connect", false).is_ok());
        for endpoint in [
            "http://127.0.0.1:5001/connect",
            "http://localhost:5001/connect",
            "http://game.example/connect",
            "https://user:password@game.example/connect",
            "https://game.example/connect#fragment",
            "file:///connect",
        ] {
            assert!(validate_endpoint(endpoint, false).is_err());
        }
        assert!(validate_endpoint("http://127.0.0.1:5001/connect", true).is_ok());
        assert!(validate_endpoint("http://localhost:5001/connect", true).is_ok());
        assert!(validate_endpoint("http://game.example/connect", true).is_err());
        assert!(validate_endpoint("http://localhost.example/connect", true).is_err());
    }

    #[test]
    fn validates_guest_responses_without_exposing_credentials() {
        let endpoint = Url::parse("https://game.example/connect").expect("endpoint");
        let address = SocketAddr::from(([127, 0, 0, 1], SERVER_PORT));
        let token = ConnectToken::build(address, PROTOCOL_ID, 42, generate_key())
            .expire_seconds(60)
            .generate()
            .expect("test token");
        let data = GuestCredentials {
            connect_token: encode_hex(&token.try_into_bytes().expect("token bytes")),
            certificate_digest: "aa".repeat(32),
        };
        let mut response = ehttp::Response {
            url: endpoint.to_string(),
            ok: true,
            status: 200,
            status_text: "OK".to_owned(),
            headers: ehttp::Headers::default(),
            bytes: serde_json::to_vec(&data).expect("response JSON"),
        };
        assert!(credentials(&response, &endpoint).is_ok());
        response.url = "http://game.example/connect".to_owned();
        assert!(credentials(&response, &endpoint).is_err());
        response.url = endpoint.to_string();
        response.status = 429;
        assert!(credentials(&response, &endpoint).is_err());
        response.status = 200;
        response.bytes = vec![b' '; 8193];
        assert!(credentials(&response, &endpoint).is_err());
        for pin in ["", "aa", &"zz".repeat(32)] {
            response.bytes = serde_json::to_vec(&GuestCredentials {
                connect_token: data.connect_token.clone(),
                certificate_digest: pin.to_owned(),
            })
            .expect("response JSON");
            assert!(credentials(&response, &endpoint).is_err());
        }
        response.bytes = serde_json::to_vec(&GuestCredentials {
            connect_token: "00".repeat(CONNECT_TOKEN_BYTES),
            certificate_digest: data.certificate_digest,
        })
        .expect("response JSON");
        assert!(credentials(&response, &endpoint).is_err());
    }
}
