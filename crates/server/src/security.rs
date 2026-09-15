use std::{ffi::OsString, fs::OpenOptions, io::Write, net::SocketAddr, path::Path};

use bevy::prelude::*;
use lightyear::netcode::{ConnectToken, generate_key};
use project_protocol::{PROTOCOL_ID, SERVER_PORT, security::encode_hex};

type Error = Box<dyn std::error::Error>;

#[derive(Resource)]
pub(super) struct ServerKey(pub [u8; 32]);

pub(super) fn load_key(path: &Path) -> Result<ServerKey, Error> {
    parse_key(&std::fs::read(path)?)
}

fn parse_key(bytes: &[u8]) -> Result<ServerKey, Error> {
    let Ok(key) = bytes.try_into() else {
        return Err("Netcode key file must contain exactly 32 bytes".into());
    };

    if key == [0; 32] {
        return Err("refusing an all-zero Netcode key".into());
    }

    Ok(ServerKey(key))
}

fn validate_server_address(address: SocketAddr) -> Result<(), Error> {
    if address.is_ipv4() && !address.ip().is_unspecified() && address.port() == SERVER_PORT {
        Ok(())
    } else {
        Err("use a reachable server IPv4 address and the configured server port".into())
    }
}

pub(super) fn issue_token(
    key: &ServerKey,
    address: SocketAddr,
    client_id: u64,
) -> Result<ConnectToken, Error> {
    validate_server_address(address)?;
    Ok(ConnectToken::build(address, PROTOCOL_ID, client_id, key.0)
        .internal_addresses(SocketAddr::from(([0, 0, 0, 0], SERVER_PORT)))?
        .expire_seconds(60)
        .timeout_seconds(15)
        .generate()?)
}

const USAGE: &str =
    "usage: project-server [generate-key KEY_FILE | issue-token KEY_FILE SERVER_IP:5000 CLIENT_ID]";

fn generate_key_file(args: &[String]) -> Result<Option<ServerKey>, Error> {
    let [path] = args else {
        return Err(USAGE.into());
    };
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

fn print_token(args: &[String]) -> Result<Option<ServerKey>, Error> {
    let [path, address, client_id] = args else {
        return Err(USAGE.into());
    };
    let key = load_key(Path::new(path))?;
    let address: SocketAddr = address.parse()?;
    let token = issue_token(&key, address, client_id.parse()?)?;
    println!("{}", encode_hex(&token.try_into_bytes()?));
    Ok(None)
}

fn configure_command(command: &str, args: &[String]) -> Result<Option<ServerKey>, Error> {
    match command {
        "generate-key" => generate_key_file(args),
        "issue-token" => print_token(args),
        _ => Err(USAGE.into()),
    }
}

fn configure_args(
    args: &[String],
    configured_key: Option<OsString>,
) -> Result<Option<ServerKey>, Error> {
    let Some((command, args)) = args.split_first() else {
        let path = configured_key.ok_or("PROJECT_NETCODE_KEY_FILE is required")?;
        return Ok(Some(load_key(Path::new(&path))?));
    };
    configure_command(command, args)
}

// Returns None after an operator command, or the key needed to run the server.
pub(super) fn configure() -> Result<Option<ServerKey>, Error> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    configure_args(&args, std::env::var_os("PROJECT_NETCODE_KEY_FILE"))
}
