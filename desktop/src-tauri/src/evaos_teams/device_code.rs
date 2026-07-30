use sha2::{Digest as _, Sha256};
use url::Url;
use zeroize::Zeroizing;

use super::DASHBOARD_ORIGIN;

pub(super) struct DeviceCodeProof {
    pub(super) verifier: Zeroizing<String>,
    pub(super) challenge: String,
}

impl DeviceCodeProof {
    pub(super) fn new() -> Self {
        let verifier = Zeroizing::new(format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        ));
        let challenge = device_code_challenge(&verifier);
        Self {
            verifier,
            challenge,
        }
    }
}

pub(super) fn normalize_device_code(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

pub(super) fn device_code_challenge(verifier: &str) -> String {
    let value = format!("desktop-device-verifier:{verifier}");
    hex::encode(Sha256::digest(value.as_bytes()))
}

pub(super) fn dashboard_login_url(
    callback: &str,
    state: &str,
    code_challenge: &str,
) -> Result<Url, String> {
    let mut url = Url::parse(&format!("{DASHBOARD_ORIGIN}/desktop-auth"))
        .map_err(|error| format!("invalid dashboard URL: {error}"))?;
    url.query_pairs_mut()
        .append_pair("desktop_callback", callback)
        .append_pair("desktop_auth_state", state)
        .append_pair("desktop_code_challenge", code_challenge)
        .append_pair("callback_scheme", "evaos-teams")
        .append_pair("switch_account", "1")
        .append_pair("prompt", "select_account");
    Ok(url)
}
