use super::*;

const IDENTITY_ROTATION_KEY_PREFIX: &str = "pending_identity_rotation:";
pub(super) const IDENTITY_ROTATION_SCHEMA: &str = "evaos.buzz_identity_rotation.v1";

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub(super) struct IdentityRotationChallenge {
    pub(super) schema_version: String,
    pub(super) rotation_id: String,
    pub(super) previous_identity_id: String,
    pub(super) membership_id: String,
    pub(super) community_id: String,
    pub(super) desktop_session_id: String,
    pub(super) replacement_public_key: String,
    pub(super) nonce: String,
    pub(super) expires_at: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct IdentityRotationChallengeResponse {
    pub(super) status: String,
    pub(super) challenge: IdentityRotationChallenge,
    pub(super) event_template: EventTemplate,
    pub(super) relay_host: String,
}

fn validate_identity_rotation_challenge(
    response: &IdentityRotationChallengeResponse,
    expected_membership_id: &str,
    expected_public_key: &str,
) -> Result<(), String> {
    if response.status != "identity_rotation_challenge_issued"
        || response.challenge.schema_version != IDENTITY_ROTATION_SCHEMA
        || response.challenge.membership_id != expected_membership_id
        || response.challenge.replacement_public_key != expected_public_key
        || response.event_template.kind != KEY_BINDING_KIND
    {
        return Err(
            "managed identity replacement challenge does not match this device".to_string(),
        );
    }
    for id in [
        &response.challenge.rotation_id,
        &response.challenge.previous_identity_id,
        &response.challenge.membership_id,
        &response.challenge.community_id,
        &response.challenge.desktop_session_id,
    ] {
        uuid::Uuid::parse_str(id).map_err(|_| {
            "managed identity replacement challenge contains an invalid identifier".to_string()
        })?;
    }
    if response.challenge.nonce.len() != 43
        || !response.challenge.nonce.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
    {
        return Err("managed identity replacement challenge nonce is invalid".to_string());
    }
    let expected_content = serde_json::to_string(&response.challenge)
        .map_err(|error| format!("could not serialize managed identity replacement: {error}"))?;
    let expected_tags = vec![
        vec!["t".to_string(), "evaos-teams-identity-rotation".to_string()],
        vec!["challenge".to_string(), response.challenge.nonce.clone()],
    ];
    if response.event_template.content != expected_content
        || response.event_template.tags != expected_tags
    {
        return Err("managed identity replacement template is not canonical".to_string());
    }
    let expires_at = chrono::DateTime::parse_from_rfc3339(&response.challenge.expires_at)
        .map_err(|_| "managed identity replacement expiry is invalid".to_string())?;
    let now = chrono::Utc::now();
    if expires_at <= now || expires_at > now + chrono::Duration::minutes(5) {
        return Err("managed identity replacement challenge has expired".to_string());
    }
    let created_at = i64::try_from(response.event_template.created_at)
        .map_err(|_| "managed identity replacement timestamp is invalid".to_string())?;
    let timestamp_skew = created_at
        .checked_sub(now.timestamp())
        .and_then(|skew| skew.checked_abs())
        .ok_or_else(|| "managed identity replacement timestamp is invalid".to_string())?;
    if timestamp_skew > 5 * 60 {
        return Err("managed identity replacement timestamp is invalid".to_string());
    }
    relay_websocket_url(&response.relay_host)?;
    Ok(())
}

pub(super) fn signed_identity_rotation_challenge(
    response: &IdentityRotationChallengeResponse,
    keys: &Keys,
    expected_membership_id: &str,
) -> Result<serde_json::Value, String> {
    validate_identity_rotation_challenge(
        response,
        expected_membership_id,
        &keys.public_key().to_hex(),
    )?;
    let tags = response
        .event_template
        .tags
        .iter()
        .cloned()
        .map(|tag| {
            Tag::parse(tag)
                .map_err(|error| format!("invalid identity replacement challenge tag: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let event = EventBuilder::new(
        Kind::Custom(response.event_template.kind),
        response.event_template.content.clone(),
    )
    .tags(tags)
    .custom_created_at(Timestamp::from(response.event_template.created_at))
    .sign_with_keys(keys)
    .map_err(|error| format!("could not sign managed identity replacement: {error}"))?;
    serde_json::to_value(event)
        .map_err(|error| format!("could not encode managed identity replacement: {error}"))
}

pub(super) fn pending_identity_rotation_key(membership_id: &str) -> Result<String, String> {
    uuid::Uuid::parse_str(membership_id)
        .map_err(|_| "managed membership identity is invalid".to_string())?;
    Ok(format!("{IDENTITY_ROTATION_KEY_PREFIX}{membership_id}"))
}

pub(super) fn staged_identity_rotation_entries(
    mut stored: HashMap<String, String>,
    membership_id: &str,
) -> Result<(HashMap<String, String>, Keys, String, String), String> {
    let staging_key = pending_identity_rotation_key(membership_id)?;
    let keys = stored
        .get(&staging_key)
        .map(|value| parse_stored_identity(value))
        .transpose()?
        .unwrap_or_else(Keys::generate);
    let encoded = encode_managed_identity(&keys)?;
    stored.insert(staging_key.clone(), encoded.clone());
    Ok((stored, keys, staging_key, encoded))
}

#[cfg(feature = "evaos-teams-managed")]
fn stage_identity_rotation_key(membership_id: &str) -> Result<Keys, String> {
    let (replacement, keys, staging_key, encoded) = staged_identity_rotation_entries(
        managed_store().load_all_readonly()?.unwrap_or_default(),
        membership_id,
    )?;
    managed_store()
        .replace_all(&replacement)
        .map_err(|_| "Could not stage a replacement identity in macOS Keychain".to_string())?;
    if !managed_store()
        .verify_stored_raw(&staging_key, &encoded)
        .map_err(|_| "Hive could not verify the staged replacement identity".to_string())?
    {
        return Err("Hive could not verify the staged replacement identity".to_string());
    }
    Ok(keys)
}

pub(super) fn validate_rotated_entitlement(
    entitlement: EvaosTeamsEntitlement,
    expected_relay: &str,
    expected_public_key: &str,
) -> Result<EvaosTeamsEntitlement, String> {
    if entitlement.relay_host != expected_relay {
        return Err("Managed identity replacement changed the server-selected relay".to_string());
    }
    validate_entitlement(&entitlement, expected_public_key)?;
    Ok(entitlement)
}

#[cfg(feature = "evaos-teams-managed")]
async fn recover_completed_identity_rotation(
    client: &reqwest::Client,
    token: &str,
    expected_membership_id: &str,
    keys: &Keys,
    expected_relay: &str,
) -> Result<EvaosTeamsEntitlement, String> {
    let public_key = keys.public_key().to_hex();
    let binding = get_identity_binding(client, token).await?;
    if binding.membership_id != expected_membership_id
        || binding.public_key.as_deref() != Some(public_key.as_str())
    {
        return Err("managed identity replacement was not completed".to_string());
    }
    let entitlement = get_remote_entitlement(client, token)
        .await
        .map_err(|_| "managed identity replacement entitlement was not available".to_string())?;
    validate_rotated_entitlement(entitlement, expected_relay, &public_key)
}

#[cfg(feature = "evaos-teams-managed")]
async fn rotate_lost_identity(
    client: &reqwest::Client,
    token: &str,
    keys: &Keys,
    expected_membership_id: &str,
) -> Result<EvaosTeamsEntitlement, String> {
    let public_key = keys.public_key().to_hex();
    let challenge: IdentityRotationChallengeResponse = post_json(
        client,
        "evaos-teams-access",
        Some(token),
        serde_json::json!({
            "action": "issue_identity_rotation_challenge",
            "replacement_public_key": public_key,
            "device_metadata": {
                "label": "Hive",
                "app_version": env!("CARGO_PKG_VERSION"),
                "platform": std::env::consts::OS,
            },
        }),
    )
    .await
    .map_err(|error| format!("Identity replacement was not available: {error}"))?;
    let signed_event =
        signed_identity_rotation_challenge(&challenge, keys, expected_membership_id)?;

    let verified: Result<EntitlementResponse, ApiFailure> = post_json(
        client,
        "evaos-teams-access",
        Some(token),
        serde_json::json!({
            "action": "verify_identity_rotation_challenge",
            "signed_event": signed_event,
        }),
    )
    .await;
    match verified {
        Ok(response) if response.status == "active" => validate_rotated_entitlement(
            response.entitlement,
            &challenge.relay_host,
            &public_key,
        ),
        Ok(_) | Err(_) => recover_completed_identity_rotation(
            client,
            token,
            expected_membership_id,
            keys,
            &challenge.relay_host,
        )
        .await
        .map_err(|_| {
            "Hive could not confirm identity replacement. The replacement key remains safely staged in Keychain; sign in again and retry."
                .to_string()
        }),
    }
}

/// Explicitly replace a lost managed Hive identity after Electric OAuth has
/// selected the account and exact-key recovery is unavailable. The replacement
/// private key is staged and read back from Keychain before the server is
/// allowed to rotate any public identity.
#[tauri::command]
pub(crate) async fn replace_lost_evaos_teams_identity(
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<EvaosTeamsAuthStatus, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state);
        Err("Hive managed login is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        let pending = state
            .pending_identity_recovery
            .lock()
            .map_err(|error| error.to_string())?
            .clone()
            .ok_or_else(|| "No pending Hive identity recovery".to_string())?;
        let keys = stage_identity_rotation_key(&pending.membership_id)?;
        if keys.public_key().to_hex() == pending.public_key {
            return Err("Replacement identity must differ from the lost identity".to_string());
        }
        let entitlement = rotate_lost_identity(
            &app_state.http_client,
            pending.session.as_str(),
            &keys,
            &pending.membership_id,
        )
        .await?;
        let status = persist_managed_credentials(
            &state,
            &app_state,
            pending.session.to_string(),
            keys,
            pending.membership_id,
            entitlement,
        )?;
        *state
            .pending_identity_recovery
            .lock()
            .map_err(|error| error.to_string())? = None;
        Ok(status)
    }
}
