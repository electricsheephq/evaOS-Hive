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
fn device_code_normalization_matches_dashboard_contract() {
    assert_eq!(normalize_device_code("aabb-ccdd eeff"), "AABBCCDDEEFF");
}
