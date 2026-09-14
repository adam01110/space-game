# Connection security

Clients use operator-issued Netcode tokens. The 32-byte signing/encryption key
exists only on the server/operator machine, never in the protocol crate or client.
Tokens expire 60 seconds after issuance (existing connections have a 15-second
inactivity timeout). Assign a distinct client ID to each concurrently connected
player.

WebTransport requires an explicit SHA-256 certificate pin: exactly 64 hexadecimal
characters without colons. Empty or malformed pins fail before connecting; a
mismatched pin fails the TLS handshake. The dangerous certificate-verification
bypass feature is disabled.

## Automated local testing

Run `just server` in one terminal, then `just client` in another once the server
is listening. The recipes create a private development key, capture the local
server's certificate fingerprint, and issue a fresh token before each client
launch. Compilation happens before token issuance. Certificate verification
remains enabled.

`just client` generates a random client ID by default. Use `just client 42` to
choose an ID explicitly; concurrent clients need distinct IDs. After restarting
`just server`, run `just client` again to use the new fingerprint and token.

Development state lives in `$XDG_STATE_HOME/project-1/dev`, defaulting to
`$HOME/.local/state/project-1/dev`. A lock prevents multiple recipe-managed
servers from sharing that state. These Bash recipes require `flock` and automate
native localhost testing only; browser and remote clients still need the manual
credential delivery described below. Do not distribute the development key.

## Operator setup

Build the binaries first so compilation does not consume the token lifetime:

```sh
cargo build -p project-server -p project-client
mkdir -p "$HOME/.local/state/project-1"
export PROJECT_NETCODE_KEY_FILE="$HOME/.local/state/project-1/netcode.key"
cargo run -p project-server -- generate-key "$PROJECT_NETCODE_KEY_FILE"
cargo run -p project-server
```

Key generation refuses to overwrite files and creates files with mode 0600 on
Unix. On other platforms, restrict access using OS file permissions. Keep the key
outside the repository. Missing, incorrectly sized, or all-zero keys prevent
server startup.

The server prints `PROJECT_SERVER_CERTIFICATE_DIGEST=...` on startup, including
release builds. It currently generates a new short-lived self-signed certificate
on every restart: distribute the new fingerprint after every restart. Restart
within 14 days to renew the certificate. This is explicit certificate pinning,
not public-CA validation or trust-on-first-use.

On the operator machine, issue a token for the server's reachable IPv4 address
(localhost example):

```sh
export PROJECT_NETCODE_KEY_FILE="$HOME/.local/state/project-1/netcode.key"
server=127.0.0.1:5000
key="$PROJECT_NETCODE_KEY_FILE"
cargo run -p project-server -- issue-token "$key" "$server" 42
```

Transfer the token and fingerprint through an authenticated, confidential
out-of-band channel. The token is a bearer credential: do not publish it, embed
it in a build, or place it in a URL. Never transfer the server key. Tokens are
bound to this server's wildcard bind address internally and the specified public
endpoint externally.

## Native client

Supply runtime environment variables `PROJECT_CONNECT_TOKEN` (the hexadecimal
token) and `PROJECT_SERVER_CERTIFICATE_DIGEST` (the operator-confirmed
fingerprint), then run `cargo run -p project-client`. Avoid saving token values
in shell history or logs. Missing or malformed configuration fails closed.

## Browser client

Before starting the WASM application, the hosting page must populate same-origin
`sessionStorage` entries named `PROJECT_CONNECT_TOKEN` and
`PROJECT_SERVER_CERTIFICATE_DIGEST`. Use a trusted input UI or authenticated
delivery channel, not URL parameters. Reload/start the application within the
token lifetime. No credentials are compiled into the WASM binary. The page must
run in a secure browser context supporting WebTransport.

## Scope

This provides admission through possession of an operator-issued token,
authenticated encrypted transport, and explicit server identity pinning. It does
not add user accounts, a token-distribution service, per-user revocation, or
application-level authorization. Protect the hosting page against XSS;
same-origin scripts can read browser credentials. Rotating the Netcode key
invalidates outstanding tokens; restarting also rotates the certificate pin.
