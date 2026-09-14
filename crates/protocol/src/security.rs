// Public connection bootstrap data. No server secrets belong in this crate.

// Bearer credentials delivered by the authenticated guest endpoint.
// Deliberately does not implement Debug to avoid accidental token logging.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuestCredentials {
    pub connect_token: String,
    pub certificate_digest: String,
}

#[must_use]
pub fn encode_hex(bytes: &[u8]) -> String {
    base16ct::lower::encode_string(bytes)
}

#[doc = "Decodes an exact-sized hexadecimal credential without accepting trailing data.\n\n# Errors\nReturns an error for invalid hex or an incorrect length."]
pub fn decode_hex<const N: usize>(text: &str) -> Result<[u8; N], base16ct::Error> {
    let mut bytes = [0; N];
    base16ct::mixed::decode(text, &mut bytes)?;
    Ok(bytes)
}
