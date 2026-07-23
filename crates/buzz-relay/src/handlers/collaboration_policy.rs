//! Server-owned authority gate for channel and DM collaboration mutations.
//!
//! Client tags never select the policy or control identity. Both come from the
//! exact host-resolved community immediately before a controlled mutation.

use std::sync::Arc;

use nostr::Event;

use buzz_core::kind::{
    KIND_DM_ADD_MEMBER, KIND_DM_OPEN, KIND_NIP29_CREATE_GROUP, KIND_NIP29_DELETE_GROUP,
    KIND_NIP29_EDIT_METADATA, KIND_NIP29_JOIN_REQUEST, KIND_NIP29_LEAVE_REQUEST,
    KIND_NIP29_PUT_USER, KIND_NIP29_REMOVE_USER,
};
use buzz_core::tenant::TenantContext;
use buzz_db::{CollaborationPolicy, CommunityCollaborationAuthority};

use crate::state::AppState;

/// Enforce the configured policy for mutations owned by the control plane.
pub async fn enforce(
    tenant: &TenantContext,
    state: &Arc<AppState>,
    event: &Event,
) -> Result<bool, String> {
    if !is_controlled_event(event) {
        return Ok(false);
    }

    let authority = state
        .db
        .get_community_collaboration_authority(tenant.community())
        .await
        .map_err(|error| format!("collaboration authority lookup failed: {error}"))?
        .ok_or_else(|| "collaboration authority unavailable".to_string())?;

    authorize(&authority, &event.pubkey.to_hex())
}

fn is_controlled_event(event: &Event) -> bool {
    let metadata_fields: Vec<&str> = event
        .tags
        .iter()
        .filter_map(|tag| tag.as_slice().first().map(String::as_str))
        .collect();
    is_controlled_mutation(event.kind.as_u16() as u32, &metadata_fields)
}

fn is_controlled_mutation(kind: u32, metadata_fields: &[&str]) -> bool {
    match kind {
        KIND_NIP29_PUT_USER
        | KIND_NIP29_REMOVE_USER
        | KIND_NIP29_CREATE_GROUP
        | KIND_NIP29_DELETE_GROUP
        | KIND_NIP29_JOIN_REQUEST
        | KIND_NIP29_LEAVE_REQUEST
        | KIND_DM_OPEN
        | KIND_DM_ADD_MEMBER => true,
        KIND_NIP29_EDIT_METADATA => metadata_fields
            .iter()
            .any(|field| matches!(*field, "archived" | "visibility")),
        _ => false,
    }
}

fn authorize(
    authority: &CommunityCollaborationAuthority,
    actor_pubkey: &str,
) -> Result<bool, String> {
    match authority.policy {
        CollaborationPolicy::Native => Ok(false),
        CollaborationPolicy::ControlPlane => match authority.owner_pubkey.as_deref() {
            Some(owner) if owner.eq_ignore_ascii_case(actor_pubkey) => Ok(true),
            Some(_) => Err(
                "restricted: collaboration mutation requires the current community control identity"
                    .to_string(),
            ),
            None => Err(
                "restricted: collaboration control identity is unavailable; refusing mutation"
                    .to_string(),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use buzz_core::kind::{KIND_DM_HIDE, KIND_NIP29_DELETE_EVENT};
    use nostr::{EventBuilder, Keys, Kind, Tag};

    fn authority(
        policy: CollaborationPolicy,
        owner: Option<&str>,
    ) -> CommunityCollaborationAuthority {
        CommunityCollaborationAuthority {
            policy,
            owner_pubkey: owner.map(str::to_string),
        }
    }

    #[test]
    fn classifies_only_control_plane_owned_mutations() {
        for kind in [
            KIND_NIP29_PUT_USER,
            KIND_NIP29_REMOVE_USER,
            KIND_NIP29_CREATE_GROUP,
            KIND_NIP29_DELETE_GROUP,
            KIND_NIP29_JOIN_REQUEST,
            KIND_NIP29_LEAVE_REQUEST,
            KIND_DM_OPEN,
            KIND_DM_ADD_MEMBER,
        ] {
            assert!(is_controlled_mutation(kind, &[]), "kind {kind}");
        }

        assert!(is_controlled_mutation(
            KIND_NIP29_EDIT_METADATA,
            &["name", "visibility"]
        ));
        assert!(is_controlled_mutation(
            KIND_NIP29_EDIT_METADATA,
            &["archived"]
        ));
        assert!(!is_controlled_mutation(
            KIND_NIP29_EDIT_METADATA,
            &["name", "about", "topic", "ttl"]
        ));
        assert!(!is_controlled_mutation(KIND_NIP29_DELETE_EVENT, &[]));
        assert!(!is_controlled_mutation(KIND_DM_HIDE, &[]));
    }

    #[test]
    fn native_policy_preserves_existing_authorization() {
        assert_eq!(
            authorize(&authority(CollaborationPolicy::Native, None), "actor"),
            Ok(false)
        );
    }

    #[test]
    fn control_plane_accepts_only_current_sole_owner() {
        let current = "aa".repeat(32);
        let stale = "bb".repeat(32);
        let configured = authority(CollaborationPolicy::ControlPlane, Some(&current));

        assert_eq!(authorize(&configured, &current), Ok(true));
        assert!(authorize(&configured, &stale).is_err());
        assert!(authorize(
            &authority(CollaborationPolicy::ControlPlane, None),
            &current
        )
        .is_err());
    }

    #[test]
    fn signed_client_event_cannot_select_native_policy_with_a_tag() {
        let actor = Keys::generate();
        let event = EventBuilder::new(Kind::Custom(KIND_NIP29_PUT_USER as u16), "")
            .tags([
                Tag::parse(["h", "00000000-0000-4000-8000-000000000001"]).unwrap(),
                Tag::parse(["p", &"11".repeat(32)]).unwrap(),
                Tag::parse(["collaboration_policy", "native"]).unwrap(),
            ])
            .sign_with_keys(&actor)
            .unwrap();

        assert!(is_controlled_event(&event));
        assert!(authorize(
            &authority(CollaborationPolicy::ControlPlane, Some(&"22".repeat(32))),
            &event.pubkey.to_hex(),
        )
        .is_err());
    }
}
