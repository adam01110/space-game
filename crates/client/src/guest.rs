mod endpoint;
mod response;

use std::time::Duration;

use crossbeam_channel::Receiver;
use lightyear::netcode::ConnectToken;

use endpoint::configured_endpoint;
use response::credentials;

pub(super) struct Credentials {
    pub token: ConnectToken,
    pub certificate_digest: String,
}

pub(super) type GuestResult = Result<Credentials, String>;

pub(super) fn request() -> Result<Receiver<GuestResult>, String> {
    let endpoint = configured_endpoint()?;
    let mut request = ehttp::Request::post(endpoint.as_str(), Vec::new());

    request.timeout = Some(Duration::from_secs(10));
    request.headers.insert("Accept", "application/json");

    let (sender, receiver) = crossbeam_channel::bounded(1);

    ehttp::fetch(request, move |response| {
        let result = response
            .map_err(|_error| {
                "Cannot reach the guest endpoint or verify its HTTPS certificate".to_owned()
            })
            .and_then(|response| credentials(&response, &endpoint));

        if let Err(_closed) = sender.send(result) {
            bevy::log::debug!("Guest response discarded after connection attempt ended");
        }
    });
    Ok(receiver)
}
