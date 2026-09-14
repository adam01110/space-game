use std::{fs::OpenOptions, io::Write, net::SocketAddr, path::Path};

use bevy::prelude::*;
use lightyear::netcode::{generate_key, ConnectToken};
use project_protocol::{security::encode_hex, PROTOCOL_ID, SERVER_PORT};

type Error = Box<dyn std::error::Error>;

#[derive(Resource)]
pub(super) struct ServerKey(pub [u8; 32]);

pub(super) fn load_key(path: &Path) -> Result<ServerKey, Error> {
    parse_key(&std::fs::read(path)?)
}

fn parse_key(bytes: &[u8]) -> Result<ServerKey, Error> {
    let key: [u8; 32] = bytes
        .try_into()
        .map_err(|_bytes| "Netcode key file must contain exactly 32 bytes")?;
    if key == [0; 32] {
        return Err("refusing an all-zero Netcode key".into());
    }
    Ok(ServerKey(key))
}

pub(super) fn issue_token(
    key: &ServerKey,
    address: SocketAddr,
    client_id: u64,
) -> Result<ConnectToken, Error> {
    if !address.is_ipv4() || address.ip().is_unspecified() || address.port() != SERVER_PORT {
        return Err("use a reachable server IPv4 address and the configured server port".into());
    }
    Ok(ConnectToken::build(address, PROTOCOL_ID, client_id, key.0)
        .internal_addresses(SocketAddr::from(([0, 0, 0, 0], SERVER_PORT)))?
        .expire_seconds(60)
        .timeout_seconds(15)
        .generate()?)
}

/// Returns None after an operator command, or the key needed to run the server.
pub(super) fn configure() -> Result<Option<ServerKey>, Error> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, path] if command == "generate-key" => {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            options.open(path)?.write_all(&generate_key())?;
            Ok(None)
        }
        [command, path, address, client_id] if command == "issue-token" => {
            let key = load_key(Path::new(path))?;
            let address: SocketAddr = address.parse()?;
            let token = issue_token(&key, address, client_id.parse()?)?;
            println!("{}", encode_hex(&token.try_into_bytes()?));
            Ok(None)
        }
        [] => {
            let path = std::env::var_os("PROJECT_NETCODE_KEY_FILE")
                .ok_or("PROJECT_NETCODE_KEY_FILE is required")?;
            Ok(Some(load_key(Path::new(&path))?))
        }
        _ => Err("usage: project-server [generate-key KEY_FILE | issue-token KEY_FILE SERVER_IP:5000 CLIENT_ID]".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn rejects_missing_malformed_and_zero_keys() {
        for bytes in [&[][..], &[1; 31], &[1; 33], &[0; 32]] {
            assert!(parse_key(bytes).is_err());
        }
        assert_eq!(parse_key(&[1; 32]).expect("valid key").0, [1; 32]);
    }

    #[test]
    fn issues_short_lived_serializable_tokens() {
        let now = || {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_secs()
        };
        let before = now();
        let key = ServerKey(generate_key());
        let address = SocketAddr::from(([127, 0, 0, 1], SERVER_PORT));
        let token = issue_token(&key, address, 42).expect("issued token");
        assert!((before + 60..=now() + 60).contains(&token.expire_timestamp()));
        let encoded = token.try_into_bytes().expect("serialized token");
        ConnectToken::try_from_bytes(&encoded).expect("deserialized token");
        for invalid in ["0.0.0.0:5000", "[::1]:5000", "127.0.0.1:5001"] {
            assert!(issue_token(&key, invalid.parse().expect("address"), 42).is_err());
        }
    }
}
