use serde::Deserialize;

#[cfg(feature = "evaos-teams-managed")]
use super::{http_api::post_json, relay_websocket_url};
use super::{validate_entitlement, EvaosTeamsEntitlement};

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

pub(super) fn validate_identity_binding(
    binding: &IdentityBinding,
    expected_membership_id: &str,
    expected_community_id: &str,
    expected_relay_host: &str,
    expected_public_key: &str,
) -> Result<(), String> {
    if binding.membership_id != expected_membership_id
        || binding.community_id != expected_community_id
        || binding.relay_host != expected_relay_host
        || binding.public_key.as_deref() != Some(expected_public_key)
    {
        return Err("Managed identity selection changed the server-selected scope".to_string());
    }
    Ok(())
}

pub(super) fn validate_entitlement_for_binding(
    binding: &IdentityBinding,
    entitlement: &EvaosTeamsEntitlement,
    expected_membership_id: &str,
    expected_public_key: &str,
) -> Result<(), String> {
    validate_identity_binding(
        binding,
        expected_membership_id,
        &entitlement.community_id,
        &entitlement.relay_host,
        expected_public_key,
    )?;
    validate_entitlement(entitlement, expected_public_key).map(|_| ())
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
