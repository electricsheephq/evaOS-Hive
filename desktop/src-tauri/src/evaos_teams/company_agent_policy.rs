use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app_state::AppState;

use super::{current_credentials, post_json, EvaosTeamsState};

const MAX_POLICY_SELECTORS: usize = 128;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub(crate) struct HiveCompanyAgentPolicy {
    pub(super) agent_instance_id: String,
    pub(super) desired_revision: u64,
    pub(super) applied_revision: u64,
    pub(super) allowed_room_ids: Vec<String>,
    pub(super) allowed_author_membership_ids: Vec<String>,
    pub(super) status: String,
    pub(super) applied_at: Option<String>,
    pub(super) last_error_code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetHiveCompanyAgentPolicyInput {
    pub(super) agent_instance_id: String,
    pub(super) expected_revision: u64,
    pub(super) allowed_room_ids: Vec<String>,
    pub(super) allowed_author_membership_ids: Vec<String>,
}

pub(super) fn validate_agent_instance_id(value: &str) -> Result<(), String> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| "Company agent policy has an invalid agent identity".to_string())
}

fn valid_room_id(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok()
        || (value.len() == 64
            && value
                .chars()
                .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase()))
}

fn validate_unique(values: &[String], valid: impl Fn(&str) -> bool) -> Result<(), String> {
    if values.len() > MAX_POLICY_SELECTORS {
        return Err("Company agent policy has too many selections".to_string());
    }
    let mut seen = HashSet::with_capacity(values.len());
    if values
        .iter()
        .any(|value| !valid(value) || !seen.insert(value.as_str()))
    {
        return Err("Company agent policy contains an invalid selection".to_string());
    }
    Ok(())
}

pub(super) fn validate_policy(policy: &HiveCompanyAgentPolicy) -> Result<(), String> {
    validate_agent_instance_id(&policy.agent_instance_id)?;
    if policy.applied_revision > policy.desired_revision
        || !matches!(policy.status.as_str(), "pending" | "applied" | "error")
        || policy
            .last_error_code
            .as_deref()
            .is_some_and(|code| code.len() > 64 || code.chars().any(char::is_control))
    {
        return Err("Company agent policy response is invalid".to_string());
    }
    validate_unique(&policy.allowed_room_ids, valid_room_id)?;
    validate_unique(&policy.allowed_author_membership_ids, |value| {
        uuid::Uuid::parse_str(value).is_ok()
    })
}

pub(super) fn validate_set_policy_input(
    input: &SetHiveCompanyAgentPolicyInput,
) -> Result<(), String> {
    validate_agent_instance_id(&input.agent_instance_id)?;
    validate_unique(&input.allowed_room_ids, valid_room_id)?;
    validate_unique(&input.allowed_author_membership_ids, |value| {
        uuid::Uuid::parse_str(value).is_ok()
    })
}

/// Read the revisioned responder policy for one server-registered Hermes agent.
/// The backend resolves both the acting admin and target company; the renderer
/// cannot supply a tenant, account, public key, or VM selector.
#[tauri::command]
pub(crate) async fn get_hive_company_agent_policy(
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
    agent_instance_id: String,
) -> Result<HiveCompanyAgentPolicy, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state, &agent_instance_id);
        Err("Hive managed agent policy is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        validate_agent_instance_id(&agent_instance_id)?;
        if !app_state
            .evaos_teams_authorized
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err("Hive access is not active".to_string());
        }
        let (session, _, logout_pending) = current_credentials(&state).await?;
        if logout_pending {
            return Err("Hive sign-out is still pending".to_string());
        }
        let policy: HiveCompanyAgentPolicy = post_json(
            &app_state.http_client,
            "company-agent-responder-policy",
            Some(&session),
            serde_json::json!({
                "action": "get_policy",
                "agent_instance_id": agent_instance_id,
            }),
        )
        .await
        .map_err(|error| format!("Company agent policy is unavailable: {error}"))?;
        validate_policy(&policy)?;
        Ok(policy)
    }
}

/// Narrow an existing registered agent's responder policy. Hermes-owned
/// instructions, model, memory, tools, profile, credentials, and private key
/// are deliberately absent from this command and its server contract.
#[tauri::command]
pub(crate) async fn set_hive_company_agent_policy(
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
    input: SetHiveCompanyAgentPolicyInput,
) -> Result<HiveCompanyAgentPolicy, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state, &input);
        Err("Hive managed agent policy is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        validate_set_policy_input(&input)?;
        if !app_state
            .evaos_teams_authorized
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err("Hive access is not active".to_string());
        }
        let (session, _, logout_pending) = current_credentials(&state).await?;
        if logout_pending {
            return Err("Hive sign-out is still pending".to_string());
        }
        let policy: HiveCompanyAgentPolicy = post_json(
            &app_state.http_client,
            "company-agent-responder-policy",
            Some(&session),
            serde_json::json!({
                "action": "set_policy",
                "agent_instance_id": input.agent_instance_id,
                "expected_revision": input.expected_revision,
                "allowed_room_ids": input.allowed_room_ids,
                "allowed_author_membership_ids": input.allowed_author_membership_ids,
            }),
        )
        .await
        .map_err(|error| format!("Company agent policy could not be saved: {error}"))?;
        validate_policy(&policy)?;
        Ok(policy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> HiveCompanyAgentPolicy {
        HiveCompanyAgentPolicy {
            agent_instance_id: "10000000-0000-4000-8000-000000000001".to_string(),
            desired_revision: 2,
            applied_revision: 1,
            allowed_room_ids: vec!["20000000-0000-4000-8000-000000000001".to_string()],
            allowed_author_membership_ids: vec![
                "30000000-0000-4000-8000-000000000001".to_string(),
            ],
            status: "pending".to_string(),
            applied_at: None,
            last_error_code: None,
        }
    }

    #[test]
    fn accepts_bounded_server_resolvable_policy() {
        assert!(validate_policy(&policy()).is_ok());
    }

    #[test]
    fn rejects_duplicate_or_unresolved_selectors() {
        let mut value = policy();
        value.allowed_author_membership_ids.push(
            "30000000-0000-4000-8000-000000000001".to_string(),
        );
        assert!(validate_policy(&value).is_err());
        value.allowed_author_membership_ids.clear();
        value.allowed_room_ids = vec!["not-a-native-room".to_string()];
        assert!(validate_policy(&value).is_err());
    }

    #[test]
    fn rejects_applied_revision_ahead_of_desired() {
        let mut value = policy();
        value.applied_revision = 3;
        assert!(validate_policy(&value).is_err());
    }
}
