use nostr::Keys;
use serde::Deserialize;

#[cfg(feature = "evaos-teams-managed")]
use super::{http_api::post_json, relay_websocket_url};
use super::{
    valid_public_key, validate_challenge, validate_entitlement, ChallengeResponse,
    EvaosTeamsEntitlement,
};

#[derive(Clone, Debug, Deserialize, PartialEq)]
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

/// How the local native key relates to the membership's canonical identity.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum ExistingNativeIdentity {
    /// First enrollment: the membership has no canonical identity yet.
    Unbound,
    /// The local key is the canonical identity.
    Ready,
    /// A readable local key that is not the canonical one. This is a recovery
    /// state and must never be rebound by the local key.
    Mismatched,
}

pub(super) fn classify_existing_native_identity(
    binding: &IdentityBinding,
    keys: &Keys,
) -> Result<ExistingNativeIdentity, String> {
    uuid::Uuid::parse_str(&binding.membership_id)
        .map_err(|_| "Electric Sheep returned an invalid membership".to_string())?;
    let Some(canonical) = binding.public_key.as_deref() else {
        return Ok(ExistingNativeIdentity::Unbound);
    };
    if !valid_public_key(canonical) {
        return Err("Electric Sheep returned an invalid canonical Hive identity".to_string());
    }
    if canonical != keys.public_key().to_hex() {
        return Ok(ExistingNativeIdentity::Mismatched);
    }
    Ok(ExistingNativeIdentity::Ready)
}

/// Return whether the membership already has a canonical native identity.
///
/// `Ok(false)` is the first-enrollment state. A mismatch is an error here: the
/// session-resume path cannot recover in place, so it must re-authenticate
/// rather than rebind a fresh local key. The managed sign-in path classifies
/// the mismatch itself and routes it to recovery.
pub(super) fn verify_existing_native_identity(
    binding: &IdentityBinding,
    keys: &Keys,
) -> Result<bool, String> {
    match classify_existing_native_identity(binding, keys)? {
        ExistingNativeIdentity::Ready => Ok(true),
        ExistingNativeIdentity::Unbound => Ok(false),
        ExistingNativeIdentity::Mismatched => Err(
            "This device's native Buzz identity does not match the canonical Hive identity"
                .to_string(),
        ),
    }
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

pub(super) fn validate_refresh(
    binding: &IdentityBinding,
    expected: &IdentityBinding,
    expected_public_key: &str,
) -> Result<(), String> {
    validate_identity_binding(
        binding,
        &expected.membership_id,
        &expected.community_id,
        &expected.relay_host,
        expected_public_key,
    )
}

pub(super) fn validate_challenge_for_binding(
    response: &ChallengeResponse,
    binding: &IdentityBinding,
    expected_public_key: &str,
) -> Result<(), String> {
    validate_challenge(response, expected_public_key, &binding.membership_id)?;
    if binding.public_key.is_some() {
        validate_identity_binding(
            binding,
            &response.challenge.membership_id,
            &response.challenge.community_id,
            &response.relay_host,
            expected_public_key,
        )
    } else if binding.community_id != response.challenge.community_id
        || binding.relay_host != response.relay_host
    {
        Err("managed key challenge changed the server-selected scope".to_string())
    } else {
        Ok(())
    }
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
