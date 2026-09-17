use url::{Host, Url};

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
    let Some(value) = std::env::var_os("SPACE_GAME_AUTH_URL") else {
        return Ok(None);
    };

    let value = value
        .into_string()
        .map_err(|_value| "SPACE_GAME_AUTH_URL must be Unicode".to_owned())?;

    validate_endpoint(&value, cfg!(feature = "dev")).map(Some)
}

#[cfg(target_family = "wasm")]
fn runtime_endpoint() -> Result<Option<Url>, String> {
    Ok(None)
}

fn embedded_endpoint() -> Result<Option<Url>, String> {
    option_env!("SPACE_GAME_AUTH_URL")
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

pub(super) fn configured_endpoint() -> Result<Url, String> {
    if let Some(endpoint) = runtime_endpoint()? {
        return Ok(endpoint);
    } else if let Some(endpoint) = embedded_endpoint()? {
        return Ok(endpoint);
    }

    default_endpoint()
}
