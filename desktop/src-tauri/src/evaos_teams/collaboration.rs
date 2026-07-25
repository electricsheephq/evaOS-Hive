use super::EvaosTeamsState;
#[cfg(feature = "evaos-teams-managed")]
use super::{current_credentials, post_json};
use crate::app_state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[cfg(feature = "evaos-teams-managed")]
async fn active_managed_session(
    state: &EvaosTeamsState,
) -> Result<zeroize::Zeroizing<String>, String> {
    let (session, _, logout_pending, logout_confirmed, _) = current_credentials(state).await?;
    if logout_pending || logout_confirmed {
        return Err("Hive is finishing sign-out".to_string());
    }
    Ok(session)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub(crate) struct HiveWorkspaceMember {
    membership_id: String,
    public_key: Option<String>,
    binding_status: String,
    display_name: String,
    email: String,
    role: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub(crate) struct HiveWorkspaceAgent {
    agent_instance_id: String,
    public_key: String,
    display_name: String,
    runtime: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub(crate) struct HiveWorkspaceRoom {
    room_id: String,
    name: Option<String>,
    description: Option<String>,
    channel_type: Option<String>,
    visibility: Option<String>,
    ttl_seconds: Option<u64>,
    archived: Option<bool>,
    human_members: Vec<String>,
    agent_instances: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub(crate) struct HiveCollaborationState {
    role: String,
    access_revision: u64,
    reconciliation_status: String,
    seat_limit: u64,
    active_seats: u64,
    pending_seats: u64,
    members: Vec<HiveWorkspaceMember>,
    agents: Vec<HiveWorkspaceAgent>,
    rooms: Vec<HiveWorkspaceRoom>,
}

#[derive(Debug, Deserialize)]
struct CollaborationStateResponse {
    status: String,
    collaboration: HiveCollaborationState,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub(crate) struct HiveManagedRoomResult {
    room_id: String,
    name: Option<String>,
    description: Option<String>,
    channel_type: String,
    visibility: Option<String>,
    ttl_seconds: Option<u64>,
    archived: Option<bool>,
    access_revision: Option<u64>,
    reconciliation_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ManagedRoomResponse {
    status: String,
    room: HiveManagedRoomResult,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HiveCreateChannelInput {
    name: String,
    description: Option<String>,
    channel_type: String,
    visibility: String,
    ttl_seconds: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HiveChannelMutationInput {
    action: String,
    room_id: String,
    name: Option<String>,
    description: Option<String>,
    visibility: Option<String>,
    #[serde(default, deserialize_with = "crate::util::double_option")]
    ttl_seconds: Option<Option<u64>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub(crate) struct HiveInvitationResult {
    invitation_id: String,
    expires_at: String,
    email_dispatch_status: String,
}

#[derive(Debug, Deserialize)]
struct InvitationResponse {
    status: String,
    invitation: HiveInvitationResult,
}

fn valid_public_key(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Load the server-derived Hive workspace directory and reconciliation state.
#[tauri::command]
pub(crate) async fn get_hive_collaboration_state(
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<HiveCollaborationState, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state);
        Err("Hive managed collaboration is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        let session = active_managed_session(&state).await?;
        let response: CollaborationStateResponse = post_json(
            &app_state.http_client,
            "evaos-teams-access",
            Some(&session),
            serde_json::json!({ "action": "get_collaboration_state" }),
        )
        .await
        .map_err(|error| format!("Hive workspace could not be loaded: {error}"))?;
        if response.status != "active" {
            return Err("Hive workspace response was not active".to_string());
        }
        Ok(response.collaboration)
    }
}

/// Request a named private stream through the Supabase-owned control plane.
#[tauri::command]
pub(crate) async fn create_hive_channel(
    input: HiveCreateChannelInput,
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<HiveManagedRoomResult, String> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("Channel name must contain between 1 and 80 characters".to_string());
    }
    let description = input
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if description.is_some_and(|value| value.chars().count() > 500) {
        return Err("Channel description must be 500 characters or fewer".to_string());
    }
    if !matches!(input.channel_type.as_str(), "stream" | "forum") {
        return Err("Channel type is invalid".to_string());
    }
    if !matches!(input.visibility.as_str(), "open" | "private") {
        return Err("Channel visibility is invalid".to_string());
    }
    if input
        .ttl_seconds
        .is_some_and(|seconds| !(1_800..=2_592_000).contains(&seconds))
    {
        return Err("Temporary channel duration is invalid".to_string());
    }

    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state, name, description);
        Err("Hive managed collaboration is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        let session = active_managed_session(&state).await?;
        let response: ManagedRoomResponse = post_json(
            &app_state.http_client,
            "evaos-teams-access",
            Some(&session),
            serde_json::json!({
                "action": "create_channel",
                "name": name,
                "description": description,
                "channel_type": input.channel_type,
                "visibility": input.visibility,
                "ttl_seconds": input.ttl_seconds,
            }),
        )
        .await
        .map_err(|error| format!("Channel could not be created: {error}"))?;
        if response.status != "accepted" {
            return Err("Channel creation was not accepted".to_string());
        }
        Ok(response.room)
    }
}

/// Join a company-visible channel through the Supabase-owned control plane.
#[tauri::command]
pub(crate) async fn join_hive_channel(
    room_id: String,
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<HiveManagedRoomResult, String> {
    uuid::Uuid::parse_str(room_id.trim()).map_err(|_| "Managed channel is invalid".to_string())?;

    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state, room_id);
        Err("Hive managed collaboration is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        let session = active_managed_session(&state).await?;
        let response: ManagedRoomResponse = post_json(
            &app_state.http_client,
            "evaos-teams-access",
            Some(&session),
            serde_json::json!({
                "action": "join_channel",
                "room_id": room_id,
            }),
        )
        .await
        .map_err(|error| format!("Channel could not be joined: {error}"))?;
        if response.status != "accepted" {
            return Err("Channel join was not accepted".to_string());
        }
        Ok(response.room)
    }
}

/// Apply one owner/admin channel lifecycle mutation through the managed broker.
#[tauri::command]
pub(crate) async fn mutate_hive_channel(
    input: HiveChannelMutationInput,
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<HiveManagedRoomResult, String> {
    uuid::Uuid::parse_str(input.room_id.trim())
        .map_err(|_| "Managed channel is invalid".to_string())?;
    if !matches!(
        input.action.as_str(),
        "update_channel"
            | "archive_channel"
            | "unarchive_channel"
            | "delete_channel"
            | "leave_channel"
    ) {
        return Err("Managed channel action is invalid".to_string());
    }
    if input.action == "update_channel" {
        if input.name.is_none()
            && input.description.is_none()
            && input.visibility.is_none()
            && input.ttl_seconds.is_none()
        {
            return Err("At least one channel field must change".to_string());
        }
        if input
            .name
            .as_deref()
            .is_some_and(|value| value.trim().is_empty() || value.chars().count() > 80)
        {
            return Err("Channel name must contain between 1 and 80 characters".to_string());
        }
        if input
            .description
            .as_deref()
            .is_some_and(|value| value.chars().count() > 500)
        {
            return Err("Channel description must be 500 characters or fewer".to_string());
        }
        if input
            .visibility
            .as_deref()
            .is_some_and(|value| !matches!(value, "open" | "private"))
        {
            return Err("Channel visibility is invalid".to_string());
        }
        if input
            .ttl_seconds
            .flatten()
            .is_some_and(|seconds| !(1_800..=2_592_000).contains(&seconds))
        {
            return Err("Temporary channel duration is invalid".to_string());
        }
    }

    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state, input);
        Err("Hive managed collaboration is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        let session = active_managed_session(&state).await?;
        let mut body = serde_json::json!({
            "action": input.action,
            "room_id": input.room_id,
        });
        let object = body
            .as_object_mut()
            .ok_or_else(|| "Managed channel request is invalid".to_string())?;
        if let Some(name) = input.name {
            object.insert("name".to_string(), serde_json::json!(name.trim()));
        }
        if let Some(description) = input.description {
            object.insert(
                "description".to_string(),
                serde_json::json!(description.trim()),
            );
        }
        if let Some(visibility) = input.visibility {
            object.insert("visibility".to_string(), serde_json::json!(visibility));
        }
        if let Some(ttl_seconds) = input.ttl_seconds {
            object.insert("ttl_seconds".to_string(), serde_json::json!(ttl_seconds));
        }
        let response: ManagedRoomResponse = post_json(
            &app_state.http_client,
            "evaos-teams-access",
            Some(&session),
            body,
        )
        .await
        .map_err(|error| format!("Channel action could not be completed: {error}"))?;
        if response.status != "accepted" {
            return Err("Channel action was not accepted".to_string());
        }
        Ok(response.room)
    }
}

/// Open a human-to-human DM through the workspace control identity.
#[tauri::command]
pub(crate) async fn open_hive_dm(
    target_public_keys: Vec<String>,
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<HiveManagedRoomResult, String> {
    let mut target_public_keys = target_public_keys
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    target_public_keys.sort();
    target_public_keys.dedup();
    if target_public_keys.is_empty()
        || target_public_keys.len() > 8
        || !target_public_keys
            .iter()
            .all(|public_key| valid_public_key(public_key))
    {
        return Err("Direct-message participants are invalid".to_string());
    }

    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state, target_public_keys);
        Err("Hive managed collaboration is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        let session = active_managed_session(&state).await?;
        let response: ManagedRoomResponse = post_json(
            &app_state.http_client,
            "evaos-teams-access",
            Some(&session),
            serde_json::json!({
                "action": "open_dm",
                "target_public_keys": target_public_keys,
            }),
        )
        .await
        .map_err(|error| format!("Direct message could not be opened: {error}"))?;
        if response.status != "accepted" {
            return Err("Direct-message request was not accepted".to_string());
        }
        Ok(response.room)
    }
}

/// Add a server-verified human or registered agent to one managed room.
#[tauri::command]
pub(crate) async fn add_hive_room_participant(
    room_id: String,
    target_public_key: String,
    participant_kind: String,
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<HiveManagedRoomResult, String> {
    uuid::Uuid::parse_str(room_id.trim()).map_err(|_| "Managed channel is invalid".to_string())?;
    let target_public_key = target_public_key.trim().to_ascii_lowercase();
    if !valid_public_key(&target_public_key) {
        return Err("Channel participant is invalid".to_string());
    }
    let action = match participant_kind.as_str() {
        "human" => "add_room_human",
        "agent" => "add_room_agent",
        _ => return Err("Channel participant kind is invalid".to_string()),
    };

    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state, action);
        Err("Hive managed collaboration is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        let session = active_managed_session(&state).await?;
        let response: ManagedRoomResponse = post_json(
            &app_state.http_client,
            "evaos-teams-access",
            Some(&session),
            serde_json::json!({
                "action": action,
                "room_id": room_id,
                "target_public_key": target_public_key,
            }),
        )
        .await
        .map_err(|error| format!("Channel participant could not be added: {error}"))?;
        if response.status != "accepted" {
            return Err("Channel participant request was not accepted".to_string());
        }
        Ok(response.room)
    }
}

/// Remove a server-verified human or registered agent from one managed room.
#[tauri::command]
pub(crate) async fn remove_hive_room_participant(
    room_id: String,
    target_public_key: String,
    participant_kind: String,
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<HiveManagedRoomResult, String> {
    uuid::Uuid::parse_str(room_id.trim()).map_err(|_| "Managed channel is invalid".to_string())?;
    let target_public_key = target_public_key.trim().to_ascii_lowercase();
    if !valid_public_key(&target_public_key) {
        return Err("Channel participant is invalid".to_string());
    }
    let action = match participant_kind.as_str() {
        "human" => "remove_room_human",
        "agent" => "remove_room_agent",
        _ => return Err("Channel participant kind is invalid".to_string()),
    };

    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state, action);
        Err("Hive managed collaboration is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        let session = active_managed_session(&state).await?;
        let response: ManagedRoomResponse = post_json(
            &app_state.http_client,
            "evaos-teams-access",
            Some(&session),
            serde_json::json!({
                "action": action,
                "room_id": room_id,
                "target_public_key": target_public_key,
            }),
        )
        .await
        .map_err(|error| format!("Channel participant could not be removed: {error}"))?;
        if response.status != "accepted" {
            return Err("Channel participant removal was not accepted".to_string());
        }
        Ok(response.room)
    }
}

/// Invite a company member using the account's server-enforced seat policy.
#[tauri::command]
pub(crate) async fn invite_hive_member(
    email: String,
    role: String,
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<HiveInvitationResult, String> {
    let email = email.trim().to_ascii_lowercase();
    if email.len() > 254 || !email.contains('@') {
        return Err("Enter a valid email address".to_string());
    }
    if !matches!(role.as_str(), "admin" | "employee" | "member") {
        return Err("Invitation role is invalid".to_string());
    }

    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state, email, role);
        Err("Hive managed collaboration is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        let session = active_managed_session(&state).await?;
        let response: InvitationResponse = post_json(
            &app_state.http_client,
            "evaos-teams-access",
            Some(&session),
            serde_json::json!({
                "action": "invite_member",
                "email": email,
                "role": role,
            }),
        )
        .await
        .map_err(|error| format!("Invitation could not be sent: {error}"))?;
        if response.status != "created" {
            return Err("Invitation was not created".to_string());
        }
        Ok(response.invitation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deserialize_channel_mutation(ttl_json: &str) -> HiveChannelMutationInput {
        serde_json::from_str(&format!(
            r#"{{"action":"update","roomId":"10000000-0000-4000-8000-000000000001"{ttl_json}}}"#
        ))
        .unwrap()
    }

    #[test]
    fn channel_mutation_ttl_distinguishes_missing_null_and_value() {
        assert_eq!(deserialize_channel_mutation("").ttl_seconds, None);
        assert_eq!(
            deserialize_channel_mutation(r#","ttlSeconds":null"#).ttl_seconds,
            Some(None)
        );
        assert_eq!(
            deserialize_channel_mutation(r#","ttlSeconds":3600"#).ttl_seconds,
            Some(Some(3600))
        );
    }
}
