use serde::Deserialize;

#[cfg(feature = "evaos-teams-managed")]
use super::{http_api::post_json, relay_websocket_url};

#[derive(Debug, Deserialize, PartialEq)]
pub(super) struct IdentityBinding {
    pub(super) membership_id: String,
    pub(super) community_id: String,
    pub(super) relay_host: String,
    pub(super) public_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct IdentityBindingResponse {
    pub(super) status: String,
    pub(super) binding: IdentityBinding,
}

#[cfg(feature = "evaos-teams-managed")]
pub(super) async fn get_identity_binding(
    client: &reqwest::Client,
    token: &str,
) -> Result<IdentityBinding, String> {
    let response: IdentityBindingResponse = post_json(
        client,
        "evaos-teams-access",
        Some(token),
        serde_json::json!({ "action": "get_identity_binding" }),
    )
    .await
    .map_err(|error| format!("Managed identity selection was not available: {error}"))?;
    if response.status != "active" {
        return Err("Managed identity selection is not active".to_string());
    }
    uuid::Uuid::parse_str(&response.binding.membership_id)
        .map_err(|_| "Managed identity selection returned an invalid membership".to_string())?;
    uuid::Uuid::parse_str(&response.binding.community_id)
        .map_err(|_| "Managed identity selection returned an invalid community".to_string())?;
    relay_websocket_url(&response.binding.relay_host)
        .map_err(|_| "Managed identity selection returned an invalid relay".to_string())?;
    Ok(response.binding)
}
