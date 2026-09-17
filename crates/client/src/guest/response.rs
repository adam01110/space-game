use lightyear::netcode::{CONNECT_TOKEN_BYTES, ConnectToken};
use space_game_protocol::security::{GuestCredentials, decode_hex};
use url::Url;

use super::{Credentials, GuestResult};

// Credentials are a few hundred bytes; anything larger is not a guest response.
const MAX_RESPONSE_BYTES: usize = 8192;

fn validate_response(response: &ehttp::Response, endpoint: &Url) -> Result<(), String> {
    // Never trust credentials obtained after a redirect, including HTTPS downgrades.
    if response.url != endpoint.as_str() {
        return Err("Guest endpoint redirected; connection refused".to_owned());
    } else if response.status != 200 {
        return Err(format!("Guest endpoint returned HTTP {}", response.status));
    } else if response.bytes.len() > MAX_RESPONSE_BYTES {
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

pub(super) fn credentials(response: &ehttp::Response, endpoint: &Url) -> GuestResult {
    validate_response(response, endpoint)?;
    decode_credentials(&response.bytes)
}
