use std::cell::RefCell;

use tauri::Manager;

use super::*;

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
    if (created_at - now.timestamp()).abs() > 5 * 60 {
        return Err("managed identity replacement timestamp is invalid".to_string());
    }
    relay_websocket_url(&response.relay_host)?;
    Ok(())
}

fn signed_identity_rotation_challenge(
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

#[cfg(feature = "evaos-teams-managed")]
fn stage_identity_rotation_key(membership_id: &str) -> Result<Keys, String> {
    let staged = RefCell::new(None);
    managed_store()
        .replace_all_checked(|fresh| {
            let (replacement, keys) =
                keychain_migration::staged_identity_rotation_entries(fresh, membership_id)?;
            *staged.borrow_mut() = Some(keys);
            Ok(replacement)
        })
        .map_err(|_| "Could not stage a replacement identity in macOS Keychain".to_string())?;
    staged
        .into_inner()
        .ok_or_else(|| "Hive could not verify the staged replacement identity".to_string())
}

fn validate_rotated_entitlement(
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

#[derive(Debug, PartialEq)]
enum IdentityRotationProgress {
    Rotate,
    Resume,
}

fn identity_rotation_progress(
    current_public_key: Option<&str>,
    pending_public_key: &str,
    replacement_public_key: &str,
) -> Result<IdentityRotationProgress, String> {
    match current_public_key {
        Some(public_key) if public_key == replacement_public_key => {
            Ok(IdentityRotationProgress::Resume)
        }
        Some(public_key)
            if public_key == pending_public_key && replacement_public_key != pending_public_key =>
        {
            Ok(IdentityRotationProgress::Rotate)
        }
        _ => Err("Managed identity changed before replacement could be confirmed".to_string()),
    }
}

#[cfg(feature = "evaos-teams-managed")]
async fn recover_completed_identity_rotation(
    client: &reqwest::Client,
    token: &str,
    expected_membership_id: &str,
    keys: &Keys,
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
    validate_entitlement(&entitlement, &public_key)?;
    Ok(entitlement)
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
        Ok(_) | Err(_) => {
            recover_completed_identity_rotation(client, token, expected_membership_id, keys)
                .await
                .map_err(|_| {
                    "Hive could not confirm identity replacement. The replacement key remains safely staged in Keychain; sign in again and retry."
                        .to_string()
                })
        }
    }
}

/// Replace a genuinely lost managed Hive identity only after a fresh Electric
/// OAuth session and an explicit user confirmation.
#[tauri::command]
pub(crate) async fn replace_lost_evaos_teams_identity(
    app: tauri::AppHandle,
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<EvaosTeamsAuthStatus, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&app, &state, &app_state);
        Err("Hive managed login is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        let pending = state
            .pending_identity_reset
            .lock()
            .map_err(|error| error.to_string())?
            .clone()
            .ok_or_else(|| "No pending Hive identity replacement".to_string())?;
        let expected_local_public_key = app_state
            .keys
            .lock()
            .map_err(|error| error.to_string())?
            .public_key()
            .to_hex();
        let keys = stage_identity_rotation_key(&pending.membership_id)?;
        let replacement_public_key = keys.public_key().to_hex();

        let current_binding =
            get_identity_binding(&app_state.http_client, pending.session.as_str()).await?;
        if current_binding.membership_id != pending.membership_id {
            return Err("Managed identity replacement changed the selected membership".to_string());
        }
        let entitlement = match identity_rotation_progress(
            current_binding.public_key.as_deref(),
            &pending.public_key,
            &replacement_public_key,
        )? {
            IdentityRotationProgress::Resume => {
                recover_completed_identity_rotation(
                    &app_state.http_client,
                    pending.session.as_str(),
                    &pending.membership_id,
                    &keys,
                )
                .await?
            }
            IdentityRotationProgress::Rotate => {
                rotate_lost_identity(
                    &app_state.http_client,
                    pending.session.as_str(),
                    &keys,
                    &pending.membership_id,
                )
                .await?
            }
        };

        let replacement_binding = IdentityBinding {
            membership_id: pending.membership_id.clone(),
            public_key: Some(replacement_public_key),
        };
        identity_custody::ensure_enrollment(
            &app_state.http_client,
            pending.session.as_str(),
            &replacement_binding,
            &entitlement,
            &keys,
        )
        .await?;

        authorization::prepare_managed_identity_recovery(&app, &app_state)?;
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("app data dir: {error}"))?;
        std::fs::create_dir_all(&data_dir)
            .map_err(|error| format!("create app data dir: {error}"))?;
        let key_path = data_dir.join("identity.key");
        let store = crate::secret_store::SecretStore::shared(crate::app_state::keyring_service());
        crate::app_state::managed_identity::persist_managed_recovered_identity(
            store,
            &app_state,
            &keys,
            &expected_local_public_key,
            &key_path,
            &data_dir,
        )?;
        let status = login_identity::persist_active_session(
            &state,
            &app_state,
            pending.session.to_string(),
            &keys,
            &pending.membership_id,
            entitlement,
        )?;
        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(keys: &Keys) -> IdentityRotationChallengeResponse {
        let challenge = IdentityRotationChallenge {
            schema_version: IDENTITY_ROTATION_SCHEMA.to_string(),
            rotation_id: "10000000-0000-4000-8000-000000000001".to_string(),
            previous_identity_id: "10000000-0000-4000-8000-000000000002".to_string(),
            membership_id: "10000000-0000-4000-8000-000000000003".to_string(),
            community_id: "10000000-0000-4000-8000-000000000004".to_string(),
            desktop_session_id: "10000000-0000-4000-8000-000000000005".to_string(),
            replacement_public_key: keys.public_key().to_hex(),
            nonce: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ".to_string(),
            expires_at: (chrono::Utc::now() + chrono::Duration::minutes(2)).to_rfc3339(),
        };
        IdentityRotationChallengeResponse {
            status: "identity_rotation_challenge_issued".to_string(),
            event_template: EventTemplate {
                kind: KEY_BINDING_KIND,
                created_at: chrono::Utc::now().timestamp() as u64,
                tags: vec![
                    vec!["t".to_string(), "evaos-teams-identity-rotation".to_string()],
                    vec!["challenge".to_string(), challenge.nonce.clone()],
                ],
                content: serde_json::to_string(&challenge).unwrap(),
            },
            challenge,
            relay_host: "https://relay.example.com".to_string(),
        }
    }

    #[test]
    fn rotation_signature_uses_only_the_exact_server_template() {
        let keys = Keys::generate();
        let response = fixture(&keys);
        let event = signed_identity_rotation_challenge(
            &response,
            &keys,
            "10000000-0000-4000-8000-000000000003",
        )
        .unwrap();
        assert_eq!(event["pubkey"], keys.public_key().to_hex());
        assert_eq!(event["content"], response.event_template.content);

        let mut altered = fixture(&keys);
        altered.event_template.tags.push(vec![
            "community".to_string(),
            altered.challenge.community_id.clone(),
        ]);
        assert!(signed_identity_rotation_challenge(
            &altered,
            &keys,
            "10000000-0000-4000-8000-000000000003",
        )
        .is_err());
    }

    #[test]
    fn completed_rotation_resumes_when_pending_state_was_rebuilt_after_restart() {
        let replacement_public_key = Keys::generate().public_key().to_hex();
        assert_eq!(
            identity_rotation_progress(
                Some(&replacement_public_key),
                &replacement_public_key,
                &replacement_public_key,
            )
            .unwrap(),
            IdentityRotationProgress::Resume
        );
    }

    #[test]
    fn unrotated_binding_selects_a_distinct_replacement_identity() {
        let lost_public_key = Keys::generate().public_key().to_hex();
        let replacement_public_key = Keys::generate().public_key().to_hex();
        assert_eq!(
            identity_rotation_progress(
                Some(&lost_public_key),
                &lost_public_key,
                &replacement_public_key,
            )
            .unwrap(),
            IdentityRotationProgress::Rotate
        );
    }
}
