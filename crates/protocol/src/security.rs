//! Encoding for out-of-band credentials. No server secrets belong in this crate.
use std::fmt::Write;

#[must_use]
pub fn encode_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

/// Decode an exact-sized hexadecimal credential without accepting trailing data.
///
/// # Errors
/// Returns an error for invalid hex or an incorrect length.
pub fn decode_hex<const N: usize>(text: &str) -> Result<[u8; N], &'static str> {
    if text.len() != N * 2 {
        return Err("incorrect credential length");
    }
    let mut bytes = [0; N];
    for (byte, [high, low]) in bytes.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
        let digit = |value: u8| match value {
            b'0'..=b'9' => Ok(value - b'0'),
            b'a'..=b'f' => Ok(value - b'a' + 10),
            b'A'..=b'F' => Ok(value - b'A' + 10),
            _ => Err("invalid hexadecimal credential"),
        };
        *byte = digit(*high)? * 16 + digit(*low)?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_require_exact_valid_hex() {
        assert_eq!(decode_hex::<2>("01aF"), Ok([1, 175]));
        for invalid in ["", "01", "01aaff", "01xz", "éaa"] {
            decode_hex::<2>(invalid).expect_err("invalid credential must be rejected");
        }
        assert_eq!(encode_hex(&[0, 255]), "00ff");
    }
}
