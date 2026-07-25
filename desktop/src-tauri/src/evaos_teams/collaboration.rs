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
    channel_type: Option<String>,
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
    channel_type: String,
    access_revision: Option<u64>,
    reconciliation_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ManagedRoomResponse {
    status: String,
    room: HiveManagedRoomResult,
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
    name: String,
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<HiveManagedRoomResult, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("Channel name must contain between 1 and 80 characters".to_string());
    }

    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state, name);
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
            serde_json::json!({ "action": "create_channel", "name": name }),
        )
        .await
        .map_err(|error| format!("Channel could not be created: {error}"))?;
        if response.status != "accepted" {
            return Err("Channel creation was not accepted".to_string());
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
