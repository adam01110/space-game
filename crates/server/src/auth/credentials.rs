use std::net::SocketAddr;

use lightyear::netcode::generate_key;

use project_protocol::security::{GuestCredentials, encode_hex};

use crate::security::{ServerKey, issue_token};

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

pub(super) fn credentials_json(
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
