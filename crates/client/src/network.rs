use std::net::{Ipv4Addr, SocketAddr};

use bevy::prelude::*;
use lightyear::{
    netcode::{client_plugin::NetcodeConfig, ConnectToken, NetcodeClient, CONNECT_TOKEN_BYTES},
    prelude::{client::*, *},
};
use project_protocol::security::decode_hex;

#[derive(Resource)]
pub(super) struct Credentials {
    token: ConnectToken,
    certificate_digest: String,
}

#[cfg(not(target_family = "wasm"))]
fn setting(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_error| format!("{name} is required and must be Unicode"))
}

#[cfg(target_family = "wasm")]
fn setting(name: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or_else(|| "browser window is unavailable".to_owned())?;
    let storage = window
        .session_storage()
        .map_err(|_error| "sessionStorage is inaccessible".to_owned())?
        .ok_or_else(|| "sessionStorage is unavailable".to_owned())?;
    storage
        .get_item(name)
        .map_err(|_error| format!("cannot read {name} from sessionStorage"))?
        .ok_or_else(|| format!("{name} is required in sessionStorage"))
}

impl Credentials {
    pub(super) fn load() -> Result<Self, String> {
        Self::parse(
            &setting("PROJECT_CONNECT_TOKEN")?,
            setting("PROJECT_SERVER_CERTIFICATE_DIGEST")?,
        )
    }

    fn parse(token: &str, certificate_digest: String) -> Result<Self, String> {
        decode_hex::<32>(&certificate_digest).map_err(str::to_owned)?;
        let bytes = decode_hex::<CONNECT_TOKEN_BYTES>(token).map_err(str::to_owned)?;
        let token = ConnectToken::try_from_bytes(&bytes)
            .map_err(|_error| "invalid Netcode connect token".to_owned())?;
        Ok(Self {
            token,
            certificate_digest,
        })
    }
}

pub(super) fn spawn_client(mut commands: Commands, credentials: Res<Credentials>) {
    let netcode = NetcodeClient::new(
        Authentication::Token(credentials.token.clone()),
        NetcodeConfig::default(),
    )
    .expect("invalid Netcode connect token");
    commands.insert_resource(PredictionManager::default());
    commands.spawn((
        Name::new("Client"),
        Client,
        ReplicationReceiver,
        Link::default(),
        LocalAddr(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0))),
        PingManager::default(),
        netcode,
        WebTransportClientIo {
            certificate_digest: credentials.certificate_digest.clone(),
            target: None,
        },
    ));
    commands.remove_resource::<Credentials>();
}

pub(super) fn connect_client(mut commands: Commands, client: Single<Entity, With<Client>>) {
    commands.trigger(Connect {
        entity: client.into_inner(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightyear::netcode::generate_key;
    use project_protocol::{security::encode_hex, PROTOCOL_ID, SERVER_PORT};

    #[test]
    fn requires_valid_pin_and_token() {
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, SERVER_PORT));
        let token = ConnectToken::build(address, PROTOCOL_ID, 42, generate_key())
            .expire_seconds(60)
            .generate()
            .expect("test token");
        let encoded = encode_hex(&token.try_into_bytes().expect("serialized token"));
        for pin in [
            String::new(),
            "aa".repeat(31),
            "aa".repeat(33),
            "zz".repeat(32),
        ] {
            assert!(Credentials::parse(&encoded, pin).is_err());
        }
        let pin = "aa".repeat(32);
        assert!(Credentials::parse("", pin.clone()).is_err());
        assert!(Credentials::parse(&"00".repeat(CONNECT_TOKEN_BYTES), pin.clone()).is_err());
        let credentials = Credentials::parse(&encoded, pin).expect("valid credentials");
        let client = NetcodeClient::new(
            Authentication::Token(credentials.token),
            NetcodeConfig::default(),
        )
        .expect("token authentication");
        assert_eq!(client.inner.server_addr(), address);
    }
}
