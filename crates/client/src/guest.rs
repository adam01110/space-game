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
    let url = Url::parse(value).map_err(|error| format!("Invalid guest endpoint URL: {error}"))?;
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
        let value = match value.into_string() {
            Ok(value) => value,
            Err(_value) => return Err("PROJECT_AUTH_URL must be Unicode".to_owned()),
        };

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
            .map_err(|error| format!("Cannot determine the page origin: {error:?}"))?;
        validate_endpoint(&format!("{origin}/connect"), cfg!(feature = "dev"))
    }

    #[cfg(all(not(target_family = "wasm"), feature = "dev"))]
    {
        validate_endpoint("http://127.0.0.1:5001/connect", true)
    }
    #[cfg(all(not(target_family = "wasm"), not(feature = "dev")))]
    {
        Err("This build has no guest endpoint configured".to_owned())
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
        .map_err(|error| format!("Invalid guest response: {error}"))?;
    let bytes = decode_hex::<CONNECT_TOKEN_BYTES>(&response.connect_token)
        .map_err(|error| format!("Invalid connection token encoding: {error}"))?;
    let token = ConnectToken::try_from_bytes(&bytes)
        .map_err(|error| format!("Invalid connection token: {error}"))?;

    decode_hex::<32>(&response.certificate_digest)
        .map_err(|error| format!("Invalid server certificate fingerprint: {error}"))?;

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
