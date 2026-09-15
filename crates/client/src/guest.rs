use std::time::Duration;

use crossbeam_channel::Receiver;
use lightyear::netcode::{CONNECT_TOKEN_BYTES, ConnectToken};
use url::{Host, Url};

use project_protocol::security::{GuestCredentials, decode_hex};

pub(super) struct Credentials {
    pub token: ConnectToken,
    pub certificate_digest: String,
}

pub(super) type GuestResult = Result<Credentials, String>;

fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        Some(Host::Domain("localhost")) => true,
        _ => false,
    }
}

fn has_valid_authority(url: &Url) -> bool {
    url.host().is_some() && url.username().is_empty() && url.password().is_none()
}

fn has_allowed_transport(url: &Url, allow_loopback_http: bool) -> bool {
    url.scheme() == "https" || (allow_loopback_http && url.scheme() == "http" && is_loopback(url))
}

fn validate_endpoint(value: &str, allow_loopback_http: bool) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|error| format!("Invalid guest endpoint URL: {error}"))?;
    if !has_valid_authority(&url)
        || url.fragment().is_some()
        || !has_allowed_transport(&url, allow_loopback_http)
    {
        return Err("Guest endpoint requires HTTPS (HTTP loopback is development-only)".to_owned());
    }
    Ok(url)
}

#[cfg(not(target_family = "wasm"))]
fn runtime_endpoint() -> Result<Option<Url>, String> {
    let Some(value) = std::env::var_os("PROJECT_AUTH_URL") else {
        return Ok(None);
    };
    let value = value
        .into_string()
        .map_err(|_value| "PROJECT_AUTH_URL must be Unicode".to_owned())?;
    validate_endpoint(&value, cfg!(feature = "dev")).map(Some)
}

#[cfg(target_family = "wasm")]
fn runtime_endpoint() -> Result<Option<Url>, String> {
    Ok(None)
}

fn embedded_endpoint() -> Result<Option<Url>, String> {
    option_env!("PROJECT_AUTH_URL")
        .map(|value| validate_endpoint(value, cfg!(feature = "dev")))
        .transpose()
}

#[cfg(target_family = "wasm")]
fn default_endpoint() -> Result<Url, String> {
    let origin = web_sys::window()
        .ok_or_else(|| "Browser window is unavailable".to_owned())?
        .location()
        .origin()
        .map_err(|error| format!("Cannot determine the page origin: {error:?}"))?;
    validate_endpoint(
        &format!("{origin}/connect"),
        cfg!(any(feature = "browser-dev", feature = "dev")),
    )
}

#[cfg(all(not(target_family = "wasm"), feature = "dev"))]
fn default_endpoint() -> Result<Url, String> {
    validate_endpoint("http://127.0.0.1:5001/connect", true)
}

#[cfg(all(not(target_family = "wasm"), not(feature = "dev")))]
fn default_endpoint() -> Result<Url, String> {
    Err("This build has no guest endpoint configured".to_owned())
}

fn configured_endpoint() -> Result<Url, String> {
    if let Some(endpoint) = runtime_endpoint()? {
        return Ok(endpoint);
    }
    if let Some(endpoint) = embedded_endpoint()? {
        return Ok(endpoint);
    }
    default_endpoint()
}

fn validate_response(response: &ehttp::Response, endpoint: &Url) -> Result<(), String> {
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
    Ok(())
}

fn decode_credentials(bytes: &[u8]) -> GuestResult {
    let response: GuestCredentials = serde_json::from_slice(bytes)
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

fn credentials(response: &ehttp::Response, endpoint: &Url) -> GuestResult {
    validate_response(response, endpoint)?;
    decode_credentials(&response.bytes)
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
