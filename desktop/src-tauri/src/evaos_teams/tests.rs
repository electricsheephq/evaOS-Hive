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

fn entitlement(public_key: Option<String>) -> EvaosTeamsEntitlement {
    EvaosTeamsEntitlement {
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key,
        role: "member".to_string(),
        access_revision: 7,
        expires_at: (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
        refresh_after_seconds: 300,
    }
}

#[test]
fn login_url_is_callback_and_one_way_verifier_bound() {
    let verifier = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let verifier_challenge = device_code_challenge(verifier);
    let url = dashboard_login_url(
        "http://127.0.0.1:4567/auth/callback",
        "state-12345678",
        &verifier_challenge,
    )
    .unwrap();
    let pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();

    assert_eq!(url.origin().ascii_serialization(), DASHBOARD_ORIGIN);
    assert_eq!(url.path(), "/desktop-auth");
    assert_eq!(
        pairs.get("desktop_auth_state").map(String::as_str),
        Some("state-12345678")
    );
    assert_eq!(
        pairs.get("desktop_code_challenge"),
        Some(&verifier_challenge)
    );
    assert_ne!(pairs.get("desktop_code_challenge").unwrap(), verifier);
    assert_eq!(
        pairs.get("desktop_callback").map(String::as_str),
        Some("http://127.0.0.1:4567/auth/callback")
    );
    assert_eq!(verifier_challenge.len(), 64);
    assert_ne!(
        verifier_challenge,
        device_code_challenge("1123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
    );
}

#[test]
fn callback_requires_exact_state_and_server_code_shape() {
    let expected_state = "state-12345678";
    let valid = HashMap::from([
        ("desktop_auth_state".to_string(), expected_state.to_string()),
        (
            "device_code".to_string(),
            "aabb-ccdd-eeff-0011-2233-4455-6677-8899".to_string(),
        ),
    ]);
    assert_eq!(
        callback_device_code(&valid, expected_state).unwrap(),
        "AABBCCDDEEFF00112233445566778899"
    );

    let mut wrong_state = valid.clone();
    wrong_state.insert("desktop_auth_state".to_string(), "other-state".to_string());
    assert!(callback_device_code(&wrong_state, expected_state).is_err());

    let mut invalid_code = valid;
    invalid_code.insert("device_code".to_string(), "short".to_string());
    assert!(callback_device_code(&invalid_code, expected_state).is_err());
}

#[test]
fn server_snake_case_decodes_and_renderer_camel_case_encodes() {
    let response: EntitlementResponse = serde_json::from_value(serde_json::json!({
        "status": "active",
        "entitlement": {
            "community_id": "10000000-0000-4000-8000-000000000003",
            "relay_host": "https://relay.example.com",
            "public_key": "a".repeat(64),
            "role": "member",
            "access_revision": 7,
            "expires_at": "2030-07-26T16:00:00Z",
            "refresh_after_seconds": 300,
            "ignored_server_field": true
        }
    }))
    .unwrap();
    let renderer = serde_json::to_value(response.entitlement).unwrap();

    assert_eq!(
        renderer["communityId"],
        "10000000-0000-4000-8000-000000000003"
    );
    assert_eq!(renderer["refreshAfterSeconds"], 300);
    assert!(renderer.get("community_id").is_none());
}

#[test]
fn challenge_signature_uses_exact_server_template_and_rejects_tampering() {
    let keys = Keys::generate();
    let response = challenge(&keys);
    let event = signed_challenge(&response, &keys, "10000000-0000-4000-8000-000000000002").unwrap();
    assert_eq!(event["kind"], KEY_BINDING_KIND);
    assert_eq!(event["content"], response.event_template.content);
    assert_eq!(
        event["tags"],
        serde_json::to_value(&response.event_template.tags).unwrap()
    );
    assert_eq!(event["pubkey"], keys.public_key().to_hex());

    let mut altered = challenge(&keys);
    altered.event_template.tags.push(vec![
        "community".to_string(),
        altered.challenge.community_id.clone(),
    ]);
    assert!(signed_challenge(&altered, &keys, "10000000-0000-4000-8000-000000000002").is_err());

    let wrong_membership = challenge(&keys);
    assert!(signed_challenge(
        &wrong_membership,
        &keys,
        "20000000-0000-4000-8000-000000000002"
    )
    .is_err());
}

#[test]
fn entitlement_validation_fails_closed_for_every_named_invariant() {
    let keys = Keys::generate();
    let public_key = keys.public_key().to_hex();
    let base = entitlement(Some(public_key.clone()));

    assert_eq!(
        validate_entitlement(&base, &public_key).unwrap(),
        "wss://relay.example.com"
    );

    let invalid = [
        EvaosTeamsEntitlement {
            community_id: "not-a-uuid".to_string(),
            ..base.clone()
        },
        EvaosTeamsEntitlement {
            public_key: Some(Keys::generate().public_key().to_hex()),
            ..base.clone()
        },
        EvaosTeamsEntitlement {
            role: " ".to_string(),
            ..base.clone()
        },
        EvaosTeamsEntitlement {
            refresh_after_seconds: 29,
            ..base.clone()
        },
        EvaosTeamsEntitlement {
            refresh_after_seconds: 3601,
            ..base.clone()
        },
        EvaosTeamsEntitlement {
            expires_at: (chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339(),
            ..base.clone()
        },
        EvaosTeamsEntitlement {
            relay_host: "https://user@relay.example.com".to_string(),
            ..base.clone()
        },
        EvaosTeamsEntitlement {
            relay_host: "https://relay.example.com/other".to_string(),
            ..base
        },
    ];

    for candidate in invalid {
        assert!(validate_entitlement(&candidate, &public_key).is_err());
    }
}

#[test]
fn verification_rejects_relay_or_community_scope_change() {
    let keys = Keys::generate();
    let public_key = keys.public_key().to_hex();
    let challenge = challenge(&keys);
    let verified = entitlement(None);

    assert!(bind_verified_entitlement(verified.clone(), &challenge, &public_key).is_ok());
    assert!(bind_verified_entitlement(
        EvaosTeamsEntitlement {
            relay_host: "https://other.example.com".to_string(),
            ..verified.clone()
        },
        &challenge,
        &public_key
    )
    .is_err());
    assert!(bind_verified_entitlement(
        EvaosTeamsEntitlement {
            community_id: "20000000-0000-4000-8000-000000000003".to_string(),
            ..verified
        },
        &challenge,
        &public_key
    )
    .is_err());
}

#[test]
fn existing_native_key_activates_without_replacing_identity() {
    let keys = Keys::generate();
    let binding = IdentityBinding {
        membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
        public_key: Some(keys.public_key().to_hex()),
    };
    assert!(verify_existing_native_identity(&binding, &keys).is_ok());

    let state = crate::app_state::build_app_state();
    *state.keys.lock().unwrap() = keys.clone();
    install_entitlement(
        &state,
        &keys,
        &entitlement(Some(keys.public_key().to_hex())),
    )
    .unwrap();

    assert_eq!(state.keys.lock().unwrap().public_key(), keys.public_key());
    assert!(state
        .evaos_teams_authorized
        .load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(
        state.relay_url_override.lock().unwrap().as_deref(),
        Some("wss://relay.example.com")
    );
}

#[test]
fn missing_or_mismatched_canonical_key_requires_restore_without_mutation() {
    let keys = Keys::generate();
    let state = crate::app_state::build_app_state();
    *state.keys.lock().unwrap() = keys.clone();
    let before_key = state.keys.lock().unwrap().public_key();
    let before_relay = state.relay_url_override.lock().unwrap().clone();

    for public_key in [None, Some(Keys::generate().public_key().to_hex())] {
        let binding = IdentityBinding {
            membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
            public_key,
        };
        let error = verify_existing_native_identity(&binding, &keys).unwrap_err();
        let status = EvaosTeamsAuthStatus::identity_restore_required(error);
        assert_eq!(status.phase, "identity_restore_required");
        assert!(!status.authenticated);
    }

    assert_eq!(state.keys.lock().unwrap().public_key(), before_key);
    assert_eq!(*state.relay_url_override.lock().unwrap(), before_relay);
    assert!(!state
        .evaos_teams_authorized
        .load(std::sync::atomic::Ordering::Acquire));
}

#[test]
fn managed_store_shape_allows_only_opaque_session_and_logout_marker() {
    let session_only =
        HashMap::from([(SESSION_KEY.to_string(), "opaque-session-value".to_string())]);
    let runtime = runtime_from_entries(Some(session_only)).unwrap();
    assert!(runtime.session.is_some());
    assert!(!runtime.logout_pending);

    let forbidden = HashMap::from([
        (SESSION_KEY.to_string(), "opaque-session-value".to_string()),
        ("identity".to_string(), "nsec1forbidden".to_string()),
    ]);
    assert!(runtime_from_entries(Some(forbidden)).is_err());
}

#[test]
fn public_status_never_serializes_backend_proof_or_credentials() {
    let status = EvaosTeamsAuthStatus::identity_restore_required("Restore identity");
    let json = serde_json::to_string(&status).unwrap();
    for forbidden in [
        "desktop_session",
        "nsec",
        "device_code",
        "verifier",
        "challenge",
        "private_key",
    ] {
        assert!(!json.contains(forbidden), "{forbidden}");
    }
}
