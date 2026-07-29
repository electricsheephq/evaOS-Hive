use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::EvaosTeamsState;
#[cfg(feature = "evaos-teams-managed")]
use super::{current_session, post_json};
use crate::app_state::AppState;

/// Renderer-safe company VM identity. Runtime and agent IDs are presentation
/// and classification metadata only; the public key remains the native Buzz
/// collaboration identity.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HiveCompanyAgent {
    pub(super) agent_instance_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) agent_id: Option<String>,
    pub(super) public_key: String,
    pub(super) display_name: String,
    pub(super) runtime: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawHiveCompanyAgent {
    pub(super) agent_instance_id: String,
    #[serde(default)]
    pub(super) agent_id: Option<String>,
    pub(super) public_key: String,
    pub(super) display_name: String,
    pub(super) runtime: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct CollaborationProjection {
    #[serde(default)]
    pub(super) agents: Vec<RawHiveCompanyAgent>,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "evaos-teams-managed")]
struct CollaborationResponse {
    status: String,
    collaboration: CollaborationProjection,
}

fn valid_bounded_text(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value.chars().all(|character| !character.is_control())
}

fn sanitized_agent_id(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        let valid = !value.is_empty()
            && value.len() <= 120
            && value.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == '-' || character == '_'
            });
        valid.then(|| value.to_string())
    })
}

pub(super) fn sanitize_company_agents(agents: Vec<RawHiveCompanyAgent>) -> Vec<HiveCompanyAgent> {
    let mut seen = HashSet::new();
    agents
        .into_iter()
        .filter_map(|agent| {
            let display_name = agent.display_name.trim();
            let runtime = agent.runtime.trim();
            let valid_public_key = agent.public_key.len() == 64
                && agent
                    .public_key
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
            if uuid::Uuid::parse_str(&agent.agent_instance_id).is_err()
                || !valid_public_key
                || !seen.insert(agent.public_key.clone())
                || !valid_bounded_text(display_name, 128)
                || !valid_bounded_text(runtime, 64)
            {
                return None;
            }

            Some(HiveCompanyAgent {
                agent_instance_id: agent.agent_instance_id,
                agent_id: sanitized_agent_id(agent.agent_id),
                public_key: agent.public_key,
                display_name: display_name.to_string(),
                runtime: runtime.to_string(),
            })
        })
        .collect()
}

/// Return only tenant-authorized public company VM identities. The caller
/// cannot select a tenant, agent, capability, room, or runtime action; the
/// opaque managed session selects the server-side catalog scope.
#[tauri::command]
pub(crate) async fn list_hive_company_agents(
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<HiveCompanyAgent>, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state);
        Ok(Vec::new())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        if !app_state
            .evaos_teams_authorized
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err("Hive access is not active".to_string());
        }
        let (session, logout_pending) = current_session(&state)?;
        if logout_pending {
            return Err("Hive sign-out is still pending".to_string());
        }
        let response: CollaborationResponse = post_json(
            &app_state.http_client,
            "evaos-teams-access",
            Some(&session),
            serde_json::json!({ "action": "get_collaboration_state" }),
        )
        .await
        .map_err(|error| format!("Company agent catalog is unavailable: {error}"))?;
        if response.status != "active" {
            return Err("Company agent catalog is inactive".to_string());
        }
        Ok(sanitize_company_agents(response.collaboration.agents))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_agent(public_key: &str) -> RawHiveCompanyAgent {
        RawHiveCompanyAgent {
            agent_instance_id: "10000000-0000-4000-8000-000000000001".to_string(),
            agent_id: Some("tars".to_string()),
            public_key: public_key.to_string(),
            display_name: " TARS ".to_string(),
            runtime: " hermes ".to_string(),
        }
    }

    #[test]
    fn sanitizes_and_deduplicates_by_normalized_public_key() {
        let public_key = "a".repeat(64);
        let mut duplicate = raw_agent(&public_key);
        duplicate.display_name = "Duplicate".to_string();

        assert_eq!(
            sanitize_company_agents(vec![raw_agent(&public_key), duplicate]),
            vec![HiveCompanyAgent {
                agent_instance_id: "10000000-0000-4000-8000-000000000001".to_string(),
                agent_id: Some("tars".to_string()),
                public_key,
                display_name: "TARS".to_string(),
                runtime: "hermes".to_string(),
            }]
        );
    }

    #[test]
    fn rejects_untrusted_identity_and_text_fields() {
        let valid = "b".repeat(64);
        let mut uppercase_key = raw_agent(&valid.to_uppercase());
        let mut invalid_instance = raw_agent(&valid);
        invalid_instance.agent_instance_id = "client-selected".to_string();
        let mut control_text = raw_agent(&"c".repeat(64));
        control_text.display_name = "TARS\nadmin".to_string();
        let mut long_runtime = raw_agent(&"d".repeat(64));
        long_runtime.runtime = "x".repeat(65);
        uppercase_key.agent_id = Some("tars/admin".to_string());

        assert!(sanitize_company_agents(vec![
            uppercase_key,
            invalid_instance,
            control_text,
            long_runtime,
        ])
        .is_empty());
    }

    #[test]
    fn drops_invalid_optional_classification_without_dropping_identity() {
        let public_key = "e".repeat(64);
        let mut agent = raw_agent(&public_key);
        agent.agent_id = Some("tars/admin".to_string());

        let sanitized = sanitize_company_agents(vec![agent]);
        assert_eq!(sanitized.len(), 1);
        assert_eq!(sanitized[0].agent_id, None);
    }

    #[test]
    fn collaboration_projection_ignores_capability_and_tenant_claims() {
        let payload = serde_json::json!({
            "agents": [{
                "agent_instance_id": "10000000-0000-4000-8000-000000000001",
                "agent_id": "tars",
                "public_key": "f".repeat(64),
                "display_name": "TARS",
                "runtime": "hermes",
                "capabilities": ["admin", "provision"],
                "tenant_id": "client-selected"
            }],
            "rooms": [{"id": "private"}],
            "seats": [{"id": "secret"}]
        });

        let projection: CollaborationProjection = serde_json::from_value(payload).unwrap();
        let serialized = serde_json::to_value(sanitize_company_agents(projection.agents)).unwrap();
        assert_eq!(
            serialized,
            serde_json::json!([{
                "agentInstanceId": "10000000-0000-4000-8000-000000000001",
                "agentId": "tars",
                "publicKey": "f".repeat(64),
                "displayName": "TARS",
                "runtime": "hermes"
            }])
        );
    }
}
