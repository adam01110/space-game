# Connection security

Clients use operator-issued Netcode tokens. The 32-byte signing/encryption key
exists only on the server/operator machine, never in the protocol crate or
client. Tokens expire 60 seconds after issuance, and existing connections have a
15-second inactivity timeout. Assign a distinct client ID to each concurrently
connected player.

WebTransport requires an explicit SHA-256 certificate pin: exactly 64
hexadecimal characters without colons. Empty or malformed pins fail before
connecting; a mismatched pin fails the TLS handshake. The certificate-
verification bypass feature is disabled.

## Automated local testing

```sh
just client
just server
```

Run the two recipes in separate terminals, in either order. The client requests
short-lived credentials from the local guest endpoint without blocking the game
loop and retries until the server is available. Certificate verification stays
enabled.

The server recipe creates a private development key when needed and requires
`flock`. Development state lives in `$XDG_STATE_HOME/space-game/dev`, defaulting
to `$HOME/.local/state/space-game/dev`, and a lock prevents multiple
recipe-managed servers from sharing it. Do not distribute the development key.

## Operator setup

Build the binaries first so compilation does not consume the token lifetime:

```sh
cargo build -p space-game-server -p space-game-client
mkdir -p "$HOME/.local/state/space-game"
export SPACE_GAME_NETCODE_KEY_FILE="$HOME/.local/state/space-game/netcode.key"
cargo run -p space-game-server -- generate-key "$SPACE_GAME_NETCODE_KEY_FILE"
cargo run -p space-game-server
```

Key generation refuses to overwrite files and creates files with mode 0600 on
Unix; restrict access with OS file permissions elsewhere. Keep the key outside
the repository. Missing, incorrectly sized, or all-zero keys prevent server
startup.

The server prints `SPACE_GAME_SERVER_CERTIFICATE_DIGEST=...` on startup,
including release builds. It generates a new short-lived self-signed certificate
on every restart, so redistribute the fingerprint after every restart, and
restart within 14 days to renew. This is explicit certificate pinning, not
public-CA validation or trust-on-first-use.

On the operator machine, issue a token for the server's reachable IPv4 address
(localhost example):

```sh
export SPACE_GAME_NETCODE_KEY_FILE="$HOME/.local/state/space-game/netcode.key"
server=127.0.0.1:5000
key="$SPACE_GAME_NETCODE_KEY_FILE"
cargo run -p space-game-server -- issue-token "$key" "$server" 42
```

Transfer the token and fingerprint through an authenticated, confidential
out-of-band channel. The token is a bearer credential: do not publish it, embed
it in a build, or place it in a URL. Never transfer the server key. Tokens are
bound internally to the server's wildcard bind address and externally to the
specified public endpoint.

## Native client

```sh
SPACE_GAME_CONNECT_TOKEN=<hex token> \
SPACE_GAME_SERVER_CERTIFICATE_DIGEST=<fingerprint> \
cargo run -p space-game-client
```

Avoid saving token values in shell history or logs. Missing or malformed
configuration fails closed.

## Browser client

Before starting the WASM application, the hosting page must populate
same-origin `sessionStorage` entries named `SPACE_GAME_CONNECT_TOKEN` and
`SPACE_GAME_SERVER_CERTIFICATE_DIGEST`, using a trusted input UI or authenticated
delivery channel rather than URL parameters. Reload or start the application
within the token lifetime. No credentials are compiled into the WASM binary. The
page must run in a secure browser context supporting WebTransport.

## Scope

This provides admission through possession of an operator-issued token,
authenticated encrypted transport, and explicit server identity pinning. It does
not add user accounts, a token-distribution service, per-user revocation, or
application-level authorization. Protect the hosting page against XSS;
same-origin scripts can read browser credentials. Rotating the Netcode key
invalidates outstanding tokens; restarting also rotates the certificate pin.
