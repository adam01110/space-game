use tii::{MimeType, RequestContext, Response, StatusCode, TiiResult};

use super::AuthState;
use super::credentials::credentials_json;

const TOO_MANY_REQUESTS: StatusCode = StatusCode::from_custom(429, "Too Many Requests");

pub(super) fn connect(state: &AuthState, request: &RequestContext) -> TiiResult<Response> {
    if let Some((status, body)) = request_error(request) {
        return response(status, body);
    } else if state.limiter.check().is_err() {
        return response(TOO_MANY_REQUESTS, b"{\"error\":\"rate limit exceeded\"}")?
            .with_header("Retry-After", "1");
    }

    credentials_response(state)
}

pub(super) fn not_found(request: &mut RequestContext) -> TiiResult<Response> {
    match request.get_path() == "/connect" {
        true => response(
            StatusCode::MethodNotAllowed,
            b"{\"error\":\"method not allowed\"}",
        ),
        false => response(StatusCode::NotFound, b"{\"error\":\"not found\"}"),
    }
}

fn credentials_response(state: &AuthState) -> TiiResult<Response> {
    match credentials_json(&state.key, state.public_address, &state.certificate_digest) {
        Ok(body) => response(StatusCode::OK, body),
        Err(error) => {
            bevy::log::error!("cannot issue guest credentials: {error}");
            response(
                StatusCode::InternalServerError,
                b"{\"error\":\"credential issuance failed\"}",
            )
        }
    }
}

fn request_error(request: &RequestContext) -> Option<(StatusCode, &'static [u8])> {
    match !request.get_query().is_empty() {
        true => Some((StatusCode::NotFound, b"{\"error\":\"not found\"}")),
        false => match request_has_body(request) {
            true => Some((
                StatusCode::ContentTooLarge,
                b"{\"error\":\"request body not allowed\"}",
            )),
            false => None,
        },
    }
}

fn request_has_body(request: &RequestContext) -> bool {
    let lengths = request.get_headers("Content-Length");
    request.get_header("Transfer-Encoding").is_some() || !matches!(lengths.as_slice(), [] | ["0"])
}

fn response(status: StatusCode, body: impl AsRef<[u8]>) -> TiiResult<Response> {
    Response::new(status)
        .with_body_slice(body)
        .with_header("Content-Type", MimeType::ApplicationJson)?
        .with_header("Cache-Control", "no-store")
}
