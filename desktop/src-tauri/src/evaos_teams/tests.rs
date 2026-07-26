use super::*;

fn challenge(keys: &Keys) -> ChallengeResponse {
    let challenge = KeyBindingChallenge {
        schema_version: KEY_BINDING_SCHEMA.to_string(),
        identity_id: "10000000-0000-4000-8000-000000000001".to_string(),
        membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        desktop_session_id: "10000000-0000-4000-8000-000000000004".to_string(),
        public_key: keys.public_key().to_hex(),
        nonce: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ".to_string(),
        expires_at: (chrono::Utc::now() + chrono::Duration::minutes(2)).to_rfc3339(),
    };
    let content = serde_json::to_string(&challenge).unwrap();
    ChallengeResponse {
        status: "challenge_issued".to_string(),
        event_template: EventTemplate {
            kind: KEY_BINDING_KIND,
            created_at: chrono::Utc::now().timestamp() as u64,
            tags: vec![
                vec!["t".to_string(), "evaos-teams-key-binding".to_string()],
                vec!["challenge".to_string(), challenge.nonce.clone()],
            ],
            content,
        },
        challenge,
        relay_host: "https://relay.example.com".to_string(),
    }
}

#[test]
fn login_url_is_account_selecting_and_callback_bound() {
    let url = dashboard_login_url(
        "http://127.0.0.1:4567/auth/callback",
        "state-12345678",
        "AABBCCDDEEFF00112233445566778899",
    )
    .unwrap();
    let pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(url.origin().ascii_serialization(), DASHBOARD_ORIGIN);
    assert_eq!(url.path(), "/desktop-auth");
    assert_eq!(pairs.get("callback_scheme").unwrap(), "evaos-teams");
    assert_eq!(pairs.get("switch_account").unwrap(), "1");
    assert_eq!(pairs.get("prompt").unwrap(), "select_account");
    assert_eq!(
        pairs.get("desktop_callback").unwrap(),
        "http://127.0.0.1:4567/auth/callback"
    );
}

#[test]
fn relay_validation_accepts_only_https_origin() {
    assert_eq!(
        relay_websocket_url("https://relay.example.com:7447").unwrap(),
        "wss://relay.example.com:7447"
    );
    for invalid in [
        "http://relay.example.com",
        "https://user@relay.example.com",
        "https://relay.example.com/path",
        "https://relay.example.com/?customer=other",
        "wss://relay.example.com",
    ] {
        assert!(relay_websocket_url(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn challenge_signature_uses_exact_server_template() {
    let keys = Keys::generate();
    let response = challenge(&keys);
    let event = signed_challenge(&response, &keys).unwrap();
    assert_eq!(event["kind"], KEY_BINDING_KIND);
    assert_eq!(event["content"], response.event_template.content);
    assert_eq!(
        event["tags"],
        serde_json::to_value(&response.event_template.tags).unwrap()
    );
    assert_eq!(event["pubkey"], keys.public_key().to_hex());
}

#[test]
fn altered_challenge_template_is_rejected() {
    let keys = Keys::generate();
    let mut response = challenge(&keys);
    response.event_template.tags.push(vec![
        "community".to_string(),
        response.challenge.community_id.clone(),
    ]);
    assert!(signed_challenge(&response, &keys).is_err());
}

#[test]
fn callback_requires_exact_state_and_normalized_code() {
    let expected_code = "AABBCCDDEEFF00112233445566778899";
    let expected_state = "state-12345678";
    let valid = HashMap::from([
        ("desktop_auth_state".to_string(), expected_state.to_string()),
        (
            "device_code".to_string(),
            "aabb-ccdd-eeff-0011-2233-4455-6677-8899".to_string(),
        ),
    ]);
    assert_eq!(
        callback_device_code(&valid, expected_state, expected_code).unwrap(),
        expected_code
    );
    let mut wrong_state = valid.clone();
    wrong_state.insert("desktop_auth_state".to_string(), "other-state".to_string());
    assert!(callback_device_code(&wrong_state, expected_state, expected_code).is_err());
    let mut wrong_code = valid;
    wrong_code.insert(
        "device_code".to_string(),
        "BBBBCCDDEEFF00112233445566778899".to_string(),
    );
    assert!(callback_device_code(&wrong_code, expected_state, expected_code).is_err());
}

#[test]
fn entitlement_rejects_wrong_key_expiry_and_relay_injection() {
    let keys = Keys::generate();
    let public_key = keys.public_key().to_hex();
    let base = EvaosTeamsEntitlement {
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: Some(public_key.clone()),
        role: "member".to_string(),
        access_revision: 7,
        expires_at: (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
        refresh_after_seconds: 300,
    };
    assert_eq!(
        validate_entitlement(&base, &public_key).unwrap(),
        "wss://relay.example.com"
    );
    assert!(validate_entitlement(&base, &Keys::generate().public_key().to_hex()).is_err());
    assert!(validate_entitlement(
        &EvaosTeamsEntitlement {
            expires_at: (chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339(),
            ..base.clone()
        },
        &public_key,
    )
    .is_err());
    assert!(validate_entitlement(
        &EvaosTeamsEntitlement {
            relay_host: "https://relay.example.com/other-community".to_string(),
            ..base
        },
        &public_key,
    )
    .is_err());
    assert!(validate_entitlement(
        &EvaosTeamsEntitlement {
            public_key: None,
            ..EvaosTeamsEntitlement {
                community_id: "10000000-0000-4000-8000-000000000003".to_string(),
                relay_host: "https://relay.example.com".to_string(),
                public_key: Some(public_key.clone()),
                role: "member".to_string(),
                access_revision: 7,
                expires_at: (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
                refresh_after_seconds: 300,
            }
        },
        &public_key,
    )
    .is_err());
}

#[test]
fn entitlement_deserializes_server_snake_case_and_serializes_renderer_camel_case() {
    let response: EntitlementResponse = serde_json::from_value(serde_json::json!({
        "status": "active",
        "entitlement": {
            "community_id": "10000000-0000-4000-8000-000000000003",
            "relay_host": "https://relay.example.com",
            "public_key": "a".repeat(64),
            "role": "member",
            "access_revision": 7,
            "expires_at": "2026-07-26T16:00:00Z",
            "refresh_after_seconds": 300,
            "assignment_status": "assigned",
            "reconciliation_status": "current"
        }
    }))
    .unwrap();
    assert_eq!(
        response.entitlement.community_id,
        "10000000-0000-4000-8000-000000000003"
    );

    let renderer = serde_json::to_value(response.entitlement).unwrap();
    assert_eq!(
        renderer["communityId"],
        "10000000-0000-4000-8000-000000000003"
    );
    assert_eq!(renderer["refreshAfterSeconds"], 300);
    assert!(renderer.get("community_id").is_none());
    assert!(renderer.get("refresh_after_seconds").is_none());

    let verification: EntitlementResponse = serde_json::from_value(serde_json::json!({
        "status": "active",
        "entitlement": {
            "community_id": "10000000-0000-4000-8000-000000000003",
            "relay_host": "https://relay.example.com",
            "role": "member",
            "access_revision": 7,
            "expires_at": "2026-07-26T16:00:00Z",
            "refresh_after_seconds": 300
        }
    }))
    .unwrap();
    assert!(verification.entitlement.public_key.is_none());
}

#[test]
fn verification_binds_the_local_key_but_refresh_requires_the_server_key() {
    let public_key = Keys::generate().public_key().to_hex();
    let verified = EvaosTeamsEntitlement {
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: None,
        role: "member".to_string(),
        access_revision: 7,
        expires_at: (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
        refresh_after_seconds: 300,
    };
    let bound =
        bind_verified_entitlement(verified, "https://relay.example.com", &public_key).unwrap();
    assert_eq!(bound.public_key.as_deref(), Some(public_key.as_str()));
    assert!(validate_entitlement(
        &EvaosTeamsEntitlement {
            public_key: None,
            ..bound
        },
        &public_key,
    )
    .is_err());
}

#[test]
fn managed_keychain_service_is_distinct_from_native_buzz() {
    assert_eq!(KEYRING_SERVICE, "evaos-teams-desktop");
    assert_ne!(KEYRING_SERVICE, "buzz-desktop");
    assert_ne!(KEYRING_SERVICE, "buzz-desktop-dev");
}

#[test]
fn public_status_never_serializes_credentials() {
    let status = EvaosTeamsAuthStatus::reauth("Sign in again".to_string());
    let json = serde_json::to_string(&status).unwrap();
    assert!(!json.contains("desktop_session"));
    assert!(!json.contains("nsec"));
    assert!(!json.contains("device_code"));
    assert!(!json.contains("email"));
}

#[test]
fn logout_retry_treats_only_missing_remote_sessions_as_complete() {
    for status in [401, 404] {
        assert!(ApiFailure {
            status: reqwest::StatusCode::from_u16(status).unwrap(),
            code: "session_missing".to_string(),
        }
        .means_session_is_absent());
    }
    for status in [400, 403, 500] {
        assert!(!ApiFailure {
            status: reqwest::StatusCode::from_u16(status).unwrap(),
            code: "retry".to_string(),
        }
        .means_session_is_absent());
    }
}

#[test]
fn device_code_normalization_matches_dashboard_contract() {
    assert_eq!(normalize_device_code("aabb-ccdd eeff"), "AABBCCDDEEFF");
}

#[test]
fn signed_out_store_preserves_identity_without_session_authority() {
    let keys = Keys::generate();
    let entries = identity_only_entries(&keys).unwrap();
    assert!(entries.contains_key(IDENTITY_KEY));
    assert!(!entries.contains_key(SESSION_KEY));
    assert!(!entries.contains_key(LOGOUT_PENDING_KEY));

    let runtime = runtime_from_entries(Some(entries)).unwrap();
    assert_eq!(
        runtime.keys.unwrap().public_key(),
        keys.public_key(),
        "logout must not rotate the durable recipient identity"
    );
    assert!(runtime.session.is_none());
    assert!(!runtime.logout_pending);
}

#[test]
fn account_switch_selects_only_the_server_bound_membership_key() {
    let membership_a = "10000000-0000-4000-8000-000000000001";
    let membership_b = "10000000-0000-4000-8000-000000000002";
    let keys_a = Keys::generate();
    let keys_b = Keys::generate();
    let stored = HashMap::from([
        (
            membership_identity_key(membership_a).unwrap(),
            encode_managed_identity(&keys_a).unwrap(),
        ),
        (
            membership_identity_key(membership_b).unwrap(),
            encode_managed_identity(&keys_b).unwrap(),
        ),
    ]);

    let selected = select_login_keys(
        &stored,
        &IdentityBinding {
            membership_id: membership_b.to_string(),
            public_key: Some(keys_b.public_key().to_hex()),
        },
    )
    .unwrap();
    assert_eq!(selected.public_key(), keys_b.public_key());
}

#[test]
fn legacy_identity_is_reused_only_when_the_server_binding_matches() {
    let membership_id = "10000000-0000-4000-8000-000000000001";
    let keys = Keys::generate();
    let stored = identity_only_entries(&keys).unwrap();

    let selected = select_login_keys(
        &stored,
        &IdentityBinding {
            membership_id: membership_id.to_string(),
            public_key: Some(keys.public_key().to_hex()),
        },
    )
    .unwrap();
    assert_eq!(selected.public_key(), keys.public_key());
}

#[test]
fn switching_to_a_new_membership_preserves_an_unmigrated_legacy_key() {
    let membership_b = "10000000-0000-4000-8000-000000000002";
    let legacy_keys = Keys::generate();
    let new_keys = Keys::generate();
    let stored = identity_only_entries(&legacy_keys).unwrap();

    let persisted =
        managed_credential_entries(stored, membership_b, &new_keys, "new-session").unwrap();
    let preserved_legacy = parse_stored_identity(persisted.get(IDENTITY_KEY).unwrap()).unwrap();
    assert_eq!(preserved_legacy.public_key(), legacy_keys.public_key());
    assert_eq!(
        parse_stored_identity(
            persisted
                .get(&membership_identity_key(membership_b).unwrap())
                .unwrap(),
        )
        .unwrap()
        .public_key(),
        new_keys.public_key(),
    );
}

#[test]
fn matching_legacy_key_is_removed_only_after_membership_migration() {
    let membership_id = "10000000-0000-4000-8000-000000000001";
    let keys = Keys::generate();
    let stored = identity_only_entries(&keys).unwrap();

    let persisted =
        managed_credential_entries(stored, membership_id, &keys, "new-session").unwrap();
    assert!(!persisted.contains_key(IDENTITY_KEY));
    assert!(persisted.contains_key(&membership_identity_key(membership_id).unwrap()));
}

#[test]
fn account_switch_never_reuses_another_memberships_key() {
    let membership_a = "10000000-0000-4000-8000-000000000001";
    let membership_b = "10000000-0000-4000-8000-000000000002";
    let keys_a = Keys::generate();
    let stored = HashMap::from([(
        membership_identity_key(membership_a).unwrap(),
        encode_managed_identity(&keys_a).unwrap(),
    )]);

    let selected = select_login_keys(
        &stored,
        &IdentityBinding {
            membership_id: membership_b.to_string(),
            public_key: None,
        },
    )
    .unwrap();
    assert_ne!(selected.public_key(), keys_a.public_key());
}

#[test]
fn active_scoped_identity_restores_with_its_membership() {
    let membership_id = "10000000-0000-4000-8000-000000000001";
    let keys = Keys::generate();
    let session = "opaque-desktop-session";
    let entries = HashMap::from([
        (
            membership_identity_key(membership_id).unwrap(),
            encode_managed_identity(&keys).unwrap(),
        ),
        (ACTIVE_MEMBERSHIP_KEY.to_string(), membership_id.to_string()),
        (SESSION_KEY.to_string(), session.to_string()),
    ]);

    let runtime = runtime_from_entries(Some(entries)).unwrap();
    assert_eq!(runtime.membership_id.as_deref(), Some(membership_id));
    assert_eq!(runtime.keys.unwrap().public_key(), keys.public_key(),);
    assert_eq!(runtime.session.unwrap().as_str(), session);
}

#[test]
fn bound_membership_without_its_private_key_fails_closed() {
    let binding = IdentityBinding {
        membership_id: "10000000-0000-4000-8000-000000000001".to_string(),
        public_key: Some("a".repeat(64)),
    };
    assert!(select_login_keys(&HashMap::new(), &binding).is_err());
}

#[test]
fn session_without_identity_fails_closed() {
    let entries = HashMap::from([(
        SESSION_KEY.to_string(),
        "opaque-desktop-session".to_string(),
    )]);
    assert!(runtime_from_entries(Some(entries)).is_err());
}

#[test]
fn company_agent_catalog_drops_invalid_and_duplicate_rows() {
    let public_key = "a".repeat(64);
    let agents = sanitize_company_agents(vec![
        RawHiveCompanyAgent {
            agent_instance_id: "10000000-0000-4000-8000-000000000001".to_string(),
            public_key: public_key.clone(),
            display_name: "  ATRIS  ".to_string(),
            runtime: " hermes ".to_string(),
        },
        RawHiveCompanyAgent {
            agent_instance_id: "10000000-0000-4000-8000-000000000002".to_string(),
            public_key,
            display_name: "duplicate".to_string(),
            runtime: "hermes".to_string(),
        },
        RawHiveCompanyAgent {
            agent_instance_id: "not-a-uuid".to_string(),
            public_key: "b".repeat(64),
            display_name: "invalid".to_string(),
            runtime: "hermes".to_string(),
        },
        RawHiveCompanyAgent {
            agent_instance_id: "10000000-0000-4000-8000-000000000003".to_string(),
            public_key: "C".repeat(64),
            display_name: "uppercase key".to_string(),
            runtime: "hermes".to_string(),
        },
    ]);

    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].display_name, "ATRIS");
    assert_eq!(agents[0].runtime, "hermes");
}

#[test]
fn public_company_agent_projection_contains_no_session_or_membership_data() {
    let agent = HiveCompanyAgent {
        agent_instance_id: "10000000-0000-4000-8000-000000000001".to_string(),
        public_key: "a".repeat(64),
        display_name: "ATRIS".to_string(),
        runtime: "hermes".to_string(),
    };
    let json = serde_json::to_string(&agent).unwrap();
    assert!(!json.contains("desktop_session"));
    assert!(!json.contains("membership"));
    assert!(!json.contains("room"));
    assert!(!json.contains("email"));
}
