use super::*;
use std::collections::HashMap;

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
    let valid = vec![
        ("desktop_auth_state".to_string(), expected_state.to_string()),
        (
            "device_code".to_string(),
            "aabb-ccdd-eeff-0011-2233-4455-6677-8899".to_string(),
        ),
    ];
    assert_eq!(
        callback_device_code(&valid, expected_state, expected_code).unwrap(),
        expected_code
    );
    let mut wrong_state = valid.clone();
    wrong_state[0].1 = "other-state".to_string();
    assert!(callback_device_code(&wrong_state, expected_state, expected_code).is_err());
    let mut wrong_code = valid.clone();
    wrong_code[1].1 = "BBBBCCDDEEFF00112233445566778899".to_string();
    assert!(callback_device_code(&wrong_code, expected_state, expected_code).is_err());
    let mut credential_bearing = valid.clone();
    credential_bearing.push(("desktop_session".to_string(), "secret".to_string()));
    assert!(callback_device_code(&credential_bearing, expected_state, expected_code).is_err());
    let duplicate = vec![valid[0].clone(), valid[0].clone(), valid[1].clone()];
    assert!(callback_device_code(&duplicate, expected_state, expected_code).is_err());
}

#[test]
fn invalid_callback_does_not_consume_valid_attempt() {
    let (sender, mut receiver) = oneshot::channel();
    let callback = LoginCallback::new(
        "state-12345678".to_string(),
        "AABBCCDDEEFF00112233445566778899".to_string(),
        sender,
    );
    let invalid = vec![
        ("desktop_auth_state".to_string(), "wrong-state".to_string()),
        (
            "device_code".to_string(),
            "AABBCCDDEEFF00112233445566778899".to_string(),
        ),
    ];
    assert!(callback.try_complete(&invalid).is_err());
    assert!(matches!(
        receiver.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
    ));

    let valid = vec![
        (
            "desktop_auth_state".to_string(),
            "state-12345678".to_string(),
        ),
        (
            "device_code".to_string(),
            "aabb-ccdd-eeff-0011-2233-4455-6677-8899".to_string(),
        ),
    ];
    callback.try_complete(&valid).unwrap();
    assert_eq!(
        receiver.try_recv().unwrap().unwrap(),
        "AABBCCDDEEFF00112233445566778899"
    );
    assert!(callback.try_complete(&valid).is_err());
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn custom_scheme_accepts_only_the_exact_login_callback_shape() {
    assert!(managed_login_callback_url(
        &Url::parse("evaos-teams://auth/callback?desktop_auth_state=state&device_code=code")
            .unwrap()
    ));
    for invalid in [
        "evaos-teams://connect/callback?desktop_auth_state=state&device_code=code",
        "evaos-teams://auth/other?desktop_auth_state=state&device_code=code",
        "evaos-teams://auth:443/callback?desktop_auth_state=state&device_code=code",
        "evaos-teams://auth/callback?desktop_auth_state=state&device_code=code#fragment",
        "buzz://auth/callback?desktop_auth_state=state&device_code=code",
    ] {
        assert!(!managed_login_callback_url(&Url::parse(invalid).unwrap()));
    }
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
        assignment_status: "unassigned".to_string(),
        reconciliation_status: "current".to_string(),
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
                assignment_status: "unassigned".to_string(),
                reconciliation_status: "current".to_string(),
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
fn verification_binds_the_local_key_but_refresh_requires_the_server_key() {
    let public_key = Keys::generate().public_key().to_hex();
    let verified = EvaosTeamsEntitlement {
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: None,
        role: "member".to_string(),
        assignment_status: "unassigned".to_string(),
        reconciliation_status: "pending".to_string(),
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
fn broker_snake_case_entitlement_deserializes_and_renderer_stays_camel_case() {
    let response: EntitlementResponse = serde_json::from_value(serde_json::json!({
        "status": "active",
        "entitlement": {
            "community_id": "10000000-0000-4000-8000-000000000003",
            "relay_host": "https://relay.example.com",
            "role": "owner",
            "assignment_status": "unassigned",
            "reconciliation_status": "pending",
            "access_revision": 7,
            "expires_at": (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
            "refresh_after_seconds": 300
        }
    }))
    .unwrap();

    assert_eq!(response.status, "active");
    assert_eq!(response.entitlement.public_key, None);
    assert_eq!(response.entitlement.reconciliation_status, "pending");

    let rendered = serde_json::to_value(response.entitlement).unwrap();
    assert_eq!(
        rendered["communityId"],
        "10000000-0000-4000-8000-000000000003"
    );
    assert_eq!(rendered["reconciliationStatus"], "pending");
    assert!(rendered.get("community_id").is_none());
}

#[test]
fn managed_projection_readiness_is_separate_from_authenticated_entitlement() {
    let keys = Keys::generate();
    let public_key = keys.public_key().to_hex();
    let base = EvaosTeamsEntitlement {
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: Some(public_key.clone()),
        role: "member".to_string(),
        assignment_status: "unassigned".to_string(),
        reconciliation_status: "pending".to_string(),
        access_revision: 7,
        expires_at: (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
        refresh_after_seconds: 300,
    };

    assert!(validate_entitlement(&base, &public_key).is_ok());
    assert!(!entitlement_access_ready(&base));
    assert!(entitlement_access_ready(&EvaosTeamsEntitlement {
        reconciliation_status: "current".to_string(),
        ..base.clone()
    }));
    assert!(!entitlement_access_ready(&EvaosTeamsEntitlement {
        role: "agent_only".to_string(),
        assignment_status: "pending".to_string(),
        reconciliation_status: "current".to_string(),
        ..base.clone()
    }));
    assert!(entitlement_access_ready(&EvaosTeamsEntitlement {
        role: "agent_only".to_string(),
        assignment_status: "assigned".to_string(),
        reconciliation_status: "current".to_string(),
        ..base
    }));
}

#[test]
fn entitlement_rejects_unknown_role_and_server_status() {
    let keys = Keys::generate();
    let public_key = keys.public_key().to_hex();
    let base = EvaosTeamsEntitlement {
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: Some(public_key.clone()),
        role: "member".to_string(),
        assignment_status: "unassigned".to_string(),
        reconciliation_status: "current".to_string(),
        access_revision: 7,
        expires_at: (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
        refresh_after_seconds: 300,
    };

    for invalid in [
        EvaosTeamsEntitlement {
            role: "relay-admin".to_string(),
            ..base.clone()
        },
        EvaosTeamsEntitlement {
            assignment_status: "ready".to_string(),
            ..base.clone()
        },
        EvaosTeamsEntitlement {
            reconciliation_status: "stale".to_string(),
            ..base.clone()
        },
    ] {
        assert!(validate_entitlement(&invalid, &public_key).is_err());
    }
}

#[test]
fn managed_keychain_service_is_distinct_from_native_buzz() {
    assert_eq!(KEYRING_SERVICE, "evaos-teams-desktop");
    assert_ne!(KEYRING_SERVICE, "buzz-desktop");
    assert_ne!(KEYRING_SERVICE, "buzz-desktop-dev");
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn managed_credential_states_are_exact_and_pending_is_removed_on_activation() {
    let keys = Keys::generate();
    let pending = managed_credential_entries(&keys, "session", true, false, None).unwrap();
    assert_eq!(pending.len(), 3);
    assert_eq!(
        pending.get(LOGOUT_PENDING_KEY).map(String::as_str),
        Some("1")
    );
    let confirmed = managed_credential_entries(&keys, "session", false, true, None).unwrap();
    assert_eq!(confirmed.len(), 3);
    assert_eq!(
        confirmed.get(LOGOUT_CONFIRMED_KEY).map(String::as_str),
        Some("1")
    );
    assert!(!confirmed.contains_key(LOGOUT_PENDING_KEY));

    let active = managed_credential_entries(&keys, "session", false, false, None).unwrap();
    assert_eq!(active.len(), 2);
    assert!(!active.contains_key(LOGOUT_PENDING_KEY));
    assert!(!active.contains_key(LOGOUT_CONFIRMED_KEY));
    assert_eq!(active.get(SESSION_KEY).map(String::as_str), Some("session"));

    let reauth_pending =
        managed_credential_entries(&keys, "new-session", true, false, Some("old-session")).unwrap();
    assert_eq!(reauth_pending.len(), 4);
    assert_eq!(
        reauth_pending.get(PREVIOUS_SESSION_KEY).map(String::as_str),
        Some("old-session")
    );
    assert!(
        managed_credential_entries(&keys, "session", true, true, None).is_err(),
        "logout phases must be mutually exclusive"
    );
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn managed_nip98_auth_requires_current_entitlement() {
    let state = crate::app_state::build_app_state();
    let url = "https://relay.example.com/query";
    assert!(
        crate::relay::build_nip98_auth_header(&reqwest::Method::POST, url, b"{}", &state).is_err()
    );
    state
        .evaos_teams_authorized
        .store(true, std::sync::atomic::Ordering::Release);
    state
        .evaos_teams_expires_at
        .store(i64::MAX, std::sync::atomic::Ordering::Release);
    assert!(
        crate::relay::build_nip98_auth_header(&reqwest::Method::POST, url, b"{}", &state).is_ok()
    );
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn managed_signing_expires_in_the_backend_and_disables_access() {
    let state = crate::app_state::build_app_state();
    state
        .evaos_teams_authorized
        .store(true, std::sync::atomic::Ordering::Release);
    state.evaos_teams_expires_at.store(
        chrono::Utc::now().timestamp() - 1,
        std::sync::atomic::Ordering::Release,
    );
    assert!(state.signing_keys().is_err());
    assert!(!state
        .evaos_teams_authorized
        .load(std::sync::atomic::Ordering::Acquire));

    state
        .evaos_teams_authorized
        .store(true, std::sync::atomic::Ordering::Release);
    state
        .evaos_teams_expires_at
        .store(i64::MAX, std::sync::atomic::Ordering::Release);
    assert!(state.signing_keys().is_ok());
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn disabling_managed_access_cancels_a_long_lived_huddle() {
    let state = crate::app_state::build_app_state();
    let cancel = tokio_util::sync::CancellationToken::new();
    {
        let mut huddle = state.huddle_state.lock().unwrap();
        huddle.phase = crate::huddle::HuddlePhase::Active;
        huddle.audio_ws_cancel = Some(cancel.clone());
    }
    state
        .evaos_teams_authorized
        .store(true, std::sync::atomic::Ordering::Release);
    state
        .evaos_teams_expires_at
        .store(i64::MAX, std::sync::atomic::Ordering::Release);

    disable_managed_access(&state);

    assert!(cancel.is_cancelled());
    assert_eq!(
        state.huddle_state.lock().unwrap().phase,
        crate::huddle::HuddlePhase::Idle
    );
    assert_eq!(
        state
            .evaos_teams_expires_at
            .load(std::sync::atomic::Ordering::Acquire),
        0
    );
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
fn device_code_normalization_matches_dashboard_contract() {
    assert_eq!(normalize_device_code("aabb-ccdd eeff"), "AABBCCDDEEFF");
}
