use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::EvaosTeamsState;
#[cfg(feature = "evaos-teams-managed")]
use super::{current_session, post_json, require_managed_authorization};
use crate::app_state::AppState;

/// Tenant-scoped classification for a public identity that must already exist
/// in Buzz's native relay agent directory. Electric never supplies profile,
/// room, routing, presence, or runtime-launch authority through this shape.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HiveCompanyAgentAuthorization {
    public_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_id: Option<String>,
    runtime: String,
}

#[derive(Debug, Deserialize)]
struct RawHiveCompanyAgentAuthorization {
    public_key: String,
    #[serde(default)]
    agent_id: Option<String>,
    runtime: String,
}

#[derive(Debug, Deserialize)]
struct CollaborationProjection {
    #[serde(default)]
    agents: Vec<RawHiveCompanyAgentAuthorization>,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "evaos-teams-managed")]
struct CollaborationResponse {
    status: String,
    collaboration: CollaborationProjection,
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

fn sanitized_runtime(value: String) -> Option<String> {
    let value = value.trim();
    let valid = !value.is_empty()
        && value.len() <= 64
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        });
    valid.then(|| value.to_string())
}

fn sanitize_company_agent_authorizations(
    agents: Vec<RawHiveCompanyAgentAuthorization>,
) -> Vec<HiveCompanyAgentAuthorization> {
    let mut seen = HashSet::new();
    agents
        .into_iter()
        .filter_map(|agent| {
            let public_key = agent.public_key.trim().to_ascii_lowercase();
            let valid_public_key = public_key.len() == 64
                && public_key
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
            let runtime = sanitized_runtime(agent.runtime)?;
            if !valid_public_key || !seen.insert(public_key.clone()) {
                return None;
            }

            Some(HiveCompanyAgentAuthorization {
                public_key,
                agent_id: sanitized_agent_id(agent.agent_id),
                runtime,
            })
        })
        .collect()
}

/// Return only the company-authorized public identity classifications selected
/// by the opaque managed session. The caller cannot select a company, relay,
/// agent, room, capability, or runtime action.
#[tauri::command]
pub(crate) async fn list_hive_company_agent_authorizations(
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<HiveCompanyAgentAuthorization>, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state);
        Ok(Vec::new())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        require_managed_authorization(&app_state)?;
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
        .map_err(|error| format!("Company agent authorization is unavailable: {error}"))?;
        if response.status != "active" {
            return Err("Company agent authorization is inactive".to_string());
        }
        Ok(sanitize_company_agent_authorizations(
            response.collaboration.agents,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(public_key: &str) -> RawHiveCompanyAgentAuthorization {
        RawHiveCompanyAgentAuthorization {
            public_key: public_key.to_string(),
            agent_id: Some("tars".to_string()),
            runtime: " hermes ".to_string(),
        }
    }

    #[test]
    fn sanitizes_and_deduplicates_public_identity_classifications() {
        let public_key = "A".repeat(64);
        let sanitized =
            sanitize_company_agent_authorizations(vec![raw(&public_key), raw(&public_key)]);

        assert_eq!(
            sanitized,
            vec![HiveCompanyAgentAuthorization {
                public_key: public_key.to_ascii_lowercase(),
                agent_id: Some("tars".to_string()),
                runtime: "hermes".to_string(),
            }]
        );
    }

    #[test]
    fn drops_invalid_identity_or_runtime_and_only_drops_invalid_optional_id() {
        let mut invalid_key = raw("not-a-key");
        invalid_key.agent_id = Some("tars".to_string());
        let mut invalid_runtime = raw(&"b".repeat(64));
        invalid_runtime.runtime = "hermes\nadmin".to_string();
        let mut invalid_id = raw(&"c".repeat(64));
        invalid_id.agent_id = Some("tars/admin".to_string());

        assert_eq!(
            sanitize_company_agent_authorizations(vec![invalid_key, invalid_runtime, invalid_id,]),
            vec![HiveCompanyAgentAuthorization {
                public_key: "c".repeat(64),
                agent_id: None,
                runtime: "hermes".to_string(),
            }]
        );
    }

    #[test]
    fn projection_ignores_profile_room_capability_and_tenant_claims() {
        let payload = serde_json::json!({
            "agents": [{
                "public_key": "d".repeat(64),
                "agent_id": "tars",
                "runtime": "hermes",
                "display_name": "Catalog TARS",
                "avatar_url": "https://catalog.invalid/avatar.png",
                "channels": ["private"],
                "capabilities": ["admin"],
                "tenant_id": "client-selected"
            }],
            "rooms": [{"id": "private"}],
            "seats": [{"id": "secret"}]
        });

        let projection: CollaborationProjection = serde_json::from_value(payload).unwrap();
        let serialized =
            serde_json::to_value(sanitize_company_agent_authorizations(projection.agents)).unwrap();
        assert_eq!(
            serialized,
            serde_json::json!([{
                "publicKey": "d".repeat(64),
                "agentId": "tars",
                "runtime": "hermes"
            }])
        );
    }
}
