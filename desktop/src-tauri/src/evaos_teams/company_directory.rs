use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HiveCompanyAgent {
    pub(super) agent_instance_id: String,
    pub(super) public_key: String,
    pub(super) display_name: String,
    pub(super) runtime: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HiveCompanyMember {
    pub(super) membership_id: String,
    pub(super) public_key: String,
    pub(super) display_name: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawHiveCompanyAgent {
    pub(super) agent_instance_id: String,
    pub(super) public_key: String,
    pub(super) display_name: String,
    pub(super) runtime: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawHiveCompanyMember {
    pub(super) membership_id: String,
    pub(super) public_key: Option<String>,
    pub(super) display_name: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct CollaborationProjection {
    #[serde(default)]
    pub(super) members: Vec<RawHiveCompanyMember>,
    #[serde(default)]
    pub(super) agents: Vec<RawHiveCompanyAgent>,
}

pub(super) fn sanitize_company_agents(agents: Vec<RawHiveCompanyAgent>) -> Vec<HiveCompanyAgent> {
    let mut seen = HashSet::new();
    agents
        .into_iter()
        .filter_map(|agent| {
            let display_name = agent.display_name.trim();
            let runtime = agent.runtime.trim();
            let valid_public_key = agent.public_key.len() == 64
                && agent.public_key.chars().all(|character| {
                    character.is_ascii_hexdigit() && !character.is_ascii_uppercase()
                });
            let valid_text = |value: &str, max: usize| {
                !value.is_empty()
                    && value.len() <= max
                    && value.chars().all(|character| !character.is_control())
            };
            if uuid::Uuid::parse_str(&agent.agent_instance_id).is_err()
                || !valid_public_key
                || !seen.insert(agent.public_key.clone())
                || !valid_text(display_name, 128)
                || !valid_text(runtime, 64)
            {
                return None;
            }
            Some(HiveCompanyAgent {
                agent_instance_id: agent.agent_instance_id,
                public_key: agent.public_key,
                display_name: display_name.to_string(),
                runtime: runtime.to_string(),
            })
        })
        .collect()
}

pub(super) fn sanitize_company_members(
    members: Vec<RawHiveCompanyMember>,
) -> Vec<HiveCompanyMember> {
    let mut seen = HashSet::new();
    members
        .into_iter()
        .filter_map(|member| {
            if uuid::Uuid::parse_str(&member.membership_id).is_err() {
                return None;
            }
            let public_key = member.public_key?;
            let display_name = member.display_name.trim();
            let valid_public_key = public_key.len() == 64
                && public_key.chars().all(|character| {
                    character.is_ascii_hexdigit() && !character.is_ascii_uppercase()
                });
            let valid_display_name = !display_name.is_empty()
                && display_name.len() <= 128
                && display_name
                    .chars()
                    .all(|character| !character.is_control());
            if !valid_public_key || !seen.insert(public_key.clone()) || !valid_display_name {
                return None;
            }
            Some(HiveCompanyMember {
                membership_id: member.membership_id,
                public_key,
                display_name: display_name.to_string(),
            })
        })
        .collect()
}
