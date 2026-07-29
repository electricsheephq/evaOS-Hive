use super::*;

fn managed_credential_entries(
    mut stored: HashMap<String, String>,
    membership_id: &str,
    keys: &Keys,
    session: &str,
) -> Result<HashMap<String, String>, String> {
    let public_key = keys.public_key();
    let migrated_legacy = stored
        .get(IDENTITY_KEY)
        .map(|value| parse_stored_identity(value))
        .transpose()?
        .is_some_and(|legacy| legacy.public_key() == public_key);
    if migrated_legacy {
        stored.remove(IDENTITY_KEY);
    }
    stored.remove(&pending_identity_rotation_key(membership_id)?);
    stored.remove(LOGOUT_PENDING_KEY);
    stored.insert(
        membership_identity_key(membership_id)?,
        encode_managed_identity(keys)?,
    );
    stored.insert(ACTIVE_MEMBERSHIP_KEY.to_string(), membership_id.to_string());
    stored.insert(SESSION_KEY.to_string(), session.to_string());
    Ok(stored)
}

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

fn identity_rotation_challenge(keys: &Keys) -> IdentityRotationChallengeResponse {
    let challenge = IdentityRotationChallenge {
        schema_version: IDENTITY_ROTATION_SCHEMA.to_string(),
        rotation_id: "10000000-0000-4000-8000-000000000005".to_string(),
        previous_identity_id: "10000000-0000-4000-8000-000000000001".to_string(),
        membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        desktop_session_id: "10000000-0000-4000-8000-000000000004".to_string(),
        replacement_public_key: keys.public_key().to_hex(),
        nonce: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQ".to_string(),
        expires_at: (chrono::Utc::now() + chrono::Duration::minutes(2)).to_rfc3339(),
    };
    let content = serde_json::to_string(&challenge).unwrap();
    IdentityRotationChallengeResponse {
        status: "identity_rotation_challenge_issued".to_string(),
        event_template: EventTemplate {
            kind: KEY_BINDING_KIND,
            created_at: chrono::Utc::now().timestamp() as u64,
            tags: vec![
                vec!["t".to_string(), "evaos-teams-identity-rotation".to_string()],
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
    let verifier = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let challenge = device_code_challenge(verifier);
    let url = dashboard_login_url(
        "http://127.0.0.1:4567/auth/callback",
        "state-12345678",
        &challenge,
    )
    .unwrap();
    let pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(url.origin().ascii_serialization(), DASHBOARD_ORIGIN);
    assert_eq!(url.path(), "/desktop-auth");
    assert_eq!(pairs.get("callback_scheme").unwrap(), "evaos-teams");
    assert_eq!(pairs.get("switch_account").unwrap(), "1");
    assert_eq!(pairs.get("prompt").unwrap(), "select_account");
    assert_eq!(pairs.get("desktop_code_challenge").unwrap(), &challenge);
    assert_ne!(pairs.get("desktop_code_challenge").unwrap(), verifier);
    assert!(!pairs.contains_key("fresh"));
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
fn identity_rotation_signature_uses_exact_dedicated_template() {
    let keys = Keys::generate();
    let response = identity_rotation_challenge(&keys);
    let event =
        signed_identity_rotation_challenge(&response, &keys, &response.challenge.membership_id)
            .unwrap();
    assert_eq!(event["kind"], KEY_BINDING_KIND);
    assert_eq!(event["content"], response.event_template.content);
    assert_eq!(
        event["tags"],
        serde_json::to_value(&response.event_template.tags).unwrap()
    );
    assert_eq!(event["pubkey"], keys.public_key().to_hex());
}

#[test]
fn altered_identity_rotation_template_is_rejected() {
    let keys = Keys::generate();
    let mut response = identity_rotation_challenge(&keys);
    response.event_template.tags.push(vec![
        "community".to_string(),
        response.challenge.community_id.clone(),
    ]);
    assert!(signed_identity_rotation_challenge(
        &response,
        &keys,
        &response.challenge.membership_id,
    )
    .is_err());
}

#[test]
fn identity_rotation_rejects_timestamp_overflow_without_panicking() {
    let keys = Keys::generate();
    let mut response = identity_rotation_challenge(&keys);
    response.event_template.created_at = u64::try_from(i64::MAX).unwrap();
    assert!(signed_identity_rotation_challenge(
        &response,
        &keys,
        &response.challenge.membership_id,
    )
    .is_err());
}

#[test]
fn rotated_entitlement_remains_pinned_to_the_signed_relay() {
    let keys = Keys::generate();
    let public_key = keys.public_key().to_hex();
    let entitlement = EvaosTeamsEntitlement {
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: Some(public_key.clone()),
        role: "member".to_string(),
        access_revision: 7,
        expires_at: (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
        refresh_after_seconds: 300,
    };
    assert!(validate_rotated_entitlement(
        entitlement,
        "10000000-0000-4000-8000-000000000003",
        "https://other-relay.example.com",
        &public_key,
    )
    .is_err());
}

#[test]
fn rotated_entitlement_remains_pinned_to_the_signed_community() {
    let keys = Keys::generate();
    let public_key = keys.public_key().to_hex();
    let entitlement = EvaosTeamsEntitlement {
        community_id: "10000000-0000-4000-8000-000000000099".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: Some(public_key.clone()),
        role: "member".to_string(),
        access_revision: 7,
        expires_at: (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
        refresh_after_seconds: 300,
    };
    assert!(validate_rotated_entitlement(
        entitlement,
        "10000000-0000-4000-8000-000000000003",
        "https://relay.example.com",
        &public_key,
    )
    .is_err());
}

#[test]
fn callback_requires_exact_state_and_a_valid_server_code() {
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

#[tokio::test]
async fn manual_backup_code_completes_only_the_current_pending_login() {
    let state = EvaosTeamsState::default();
    let (sender, receiver) = oneshot::channel();
    let callback = std::sync::Arc::new(LoginCallback {
        expected_state: "state-12345678".to_string(),
        sender: Mutex::new(Some(sender)),
    });
    let registration = register_pending_login(&state, callback).unwrap();

    submit_pending_login_code(&state, "aabb-ccdd-eeff-0011").unwrap();
    assert_eq!(receiver.await.unwrap().unwrap(), "AABBCCDDEEFF0011");
    assert!(submit_pending_login_code(&state, "aabb-ccdd-eeff-0011").is_err());

    drop(registration);
    assert!(submit_pending_login_code(&state, "aabb-ccdd-eeff-0011").is_err());
}

#[test]
fn manual_backup_code_rejects_invalid_or_unpaired_input() {
    let state = EvaosTeamsState::default();
    assert!(submit_pending_login_code(&state, "short").is_err());
    assert!(submit_pending_login_code(&state, "aabb-ccdd-eeff-0011").is_err());
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
fn device_code_verifier_is_bound_by_a_one_way_challenge() {
    let verifier = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let challenge = device_code_challenge(verifier);
    assert_eq!(challenge.len(), 64);
    assert!(challenge
        .chars()
        .all(|character| character.is_ascii_hexdigit()));
    assert_ne!(challenge, verifier);
    assert_ne!(
        challenge,
        device_code_challenge("1123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
    );
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

    let selected = match select_login_keys(
        &stored,
        &IdentityBinding {
            membership_id: membership_b.to_string(),
            public_key: Some(keys_b.public_key().to_hex()),
        },
    )
    .unwrap()
    {
        LoginKeySelection::Ready(keys) => keys,
        LoginKeySelection::RecoveryRequired { .. } => panic!("matching key should be ready"),
    };
    assert_eq!(selected.public_key(), keys_b.public_key());
}

#[test]
fn legacy_identity_is_reused_only_when_the_server_binding_matches() {
    let membership_id = "10000000-0000-4000-8000-000000000001";
    let keys = Keys::generate();
    let stored = identity_only_entries(&keys).unwrap();

    let selected = match select_login_keys(
        &stored,
        &IdentityBinding {
            membership_id: membership_id.to_string(),
            public_key: Some(keys.public_key().to_hex()),
        },
    )
    .unwrap()
    {
        LoginKeySelection::Ready(keys) => keys,
        LoginKeySelection::RecoveryRequired { .. } => panic!("matching key should be ready"),
    };
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

    let selected = match select_login_keys(
        &stored,
        &IdentityBinding {
            membership_id: membership_b.to_string(),
            public_key: None,
        },
    )
    .unwrap()
    {
        LoginKeySelection::Ready(keys) => keys,
        LoginKeySelection::RecoveryRequired { .. } => {
            panic!("unbound membership should generate a fresh key")
        }
    };
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
fn bound_membership_without_its_private_key_requires_recovery() {
    let public_key = "a".repeat(64);
    let binding = IdentityBinding {
        membership_id: "10000000-0000-4000-8000-000000000001".to_string(),
        public_key: Some(public_key.clone()),
    };
    match select_login_keys(&HashMap::new(), &binding).unwrap() {
        LoginKeySelection::RecoveryRequired {
            public_key: selected,
        } => assert_eq!(selected, public_key),
        LoginKeySelection::Ready(_) => panic!("missing bound key must not be generated"),
    }
}

#[test]
fn staged_rotation_key_is_selected_only_after_the_server_binding_matches() {
    let membership_id = "10000000-0000-4000-8000-000000000001";
    let old_public_key = "a".repeat(64);
    let (stored, staged, _, _) =
        staged_identity_rotation_entries(HashMap::new(), membership_id).unwrap();

    match select_login_keys(
        &stored,
        &IdentityBinding {
            membership_id: membership_id.to_string(),
            public_key: Some(old_public_key.clone()),
        },
    )
    .unwrap()
    {
        LoginKeySelection::RecoveryRequired { public_key } => {
            assert_eq!(public_key, old_public_key)
        }
        LoginKeySelection::Ready(_) => {
            panic!("staged key must not replace an unmatched server binding")
        }
    }

    let selected = match select_login_keys(
        &stored,
        &IdentityBinding {
            membership_id: membership_id.to_string(),
            public_key: Some(staged.public_key().to_hex()),
        },
    )
    .unwrap()
    {
        LoginKeySelection::Ready(keys) => keys,
        LoginKeySelection::RecoveryRequired { .. } => {
            panic!("matching staged replacement should recover after server rotation")
        }
    };
    assert_eq!(selected.public_key(), staged.public_key());
}

#[test]
fn staged_rotation_key_is_reused_and_removed_only_on_canonical_promotion() {
    let membership_id = "10000000-0000-4000-8000-000000000001";
    let (stored, first, staging_key, encoded) =
        staged_identity_rotation_entries(HashMap::new(), membership_id).unwrap();
    assert_eq!(stored.get(&staging_key), Some(&encoded));

    let (stored, second, second_staging_key, second_encoded) =
        staged_identity_rotation_entries(stored, membership_id).unwrap();
    assert_eq!(first.public_key(), second.public_key());
    assert_eq!(staging_key, second_staging_key);
    assert_eq!(encoded, second_encoded);

    let promoted =
        managed_credential_entries(stored, membership_id, &second, "new-session").unwrap();
    assert!(!promoted.contains_key(&staging_key));
    assert_eq!(
        parse_stored_identity(
            promoted
                .get(&membership_identity_key(membership_id).unwrap())
                .unwrap(),
        )
        .unwrap()
        .public_key(),
        second.public_key(),
    );
}

#[test]
#[cfg(feature = "evaos-teams-managed")]
fn pending_identity_recovery_status_exposes_no_secret_material() {
    let state = EvaosTeamsState::default();
    let public_key = "b".repeat(64);
    let status = set_pending_identity_recovery(
        &state,
        "opaque-desktop-session".to_string(),
        IdentityBinding {
            membership_id: "10000000-0000-4000-8000-000000000001".to_string(),
            public_key: Some(public_key.clone()),
        },
        public_key,
    )
    .unwrap();

    assert_eq!(status.phase, "identity_recovery_required");
    assert!(!status.authenticated);
    let message = status.message.unwrap();
    assert!(!message.contains("opaque-desktop-session"));
    assert!(!message.contains("nsec"));
    assert!(pending_identity_recovery_status(&state).is_some());
}

#[test]
#[cfg(feature = "evaos-teams-managed")]
fn recovery_payload_accepts_only_the_server_bound_identity() {
    let keys = Keys::generate();
    let expected_public_key = keys.public_key().to_hex();
    let nsec = encode_managed_identity(&keys).unwrap();
    let payload = serde_json::json!({
        "relayUrl": "https://attacker.example.com",
        "pubkey": expected_public_key,
        "nsec": nsec,
    });

    let recovered = recovery_payload_keys(
        PayloadType::Custom,
        Zeroizing::new(payload.to_string()),
        &expected_public_key,
    )
    .unwrap();
    assert_eq!(recovered.public_key(), keys.public_key());
}

#[test]
#[cfg(feature = "evaos-teams-managed")]
fn recovery_payload_rejects_a_different_public_identity() {
    let keys = Keys::generate();
    let other_keys = Keys::generate();
    let payload = serde_json::json!({
        "pubkey": other_keys.public_key().to_hex(),
        "nsec": encode_managed_identity(&other_keys).unwrap(),
    });

    let error = recovery_payload_keys(
        PayloadType::Custom,
        Zeroizing::new(payload.to_string()),
        &keys.public_key().to_hex(),
    )
    .unwrap_err();
    assert!(error.contains("different identity"));
}

#[test]
#[cfg(feature = "evaos-teams-managed")]
fn recovery_payload_rejects_unsupported_payload_types() {
    let keys = Keys::generate();
    let error = recovery_payload_keys(
        PayloadType::Bunker,
        Zeroizing::new("nostrconnect://example".to_string()),
        &keys.public_key().to_hex(),
    )
    .unwrap_err();
    assert!(error.contains("unsupported"));
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
fn company_member_catalog_drops_unbound_invalid_and_duplicate_rows() {
    let public_key = "a".repeat(64);
    let members = sanitize_company_members(vec![
        RawHiveCompanyMember {
            membership_id: "10000000-0000-4000-8000-000000000001".to_string(),
            public_key: Some(public_key.clone()),
            display_name: "  Andrew  ".to_string(),
        },
        RawHiveCompanyMember {
            membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
            public_key: Some(public_key),
            display_name: "duplicate".to_string(),
        },
        RawHiveCompanyMember {
            membership_id: "10000000-0000-4000-8000-000000000003".to_string(),
            public_key: None,
            display_name: "not enrolled".to_string(),
        },
        RawHiveCompanyMember {
            membership_id: "10000000-0000-4000-8000-000000000004".to_string(),
            public_key: Some("B".repeat(64)),
            display_name: "uppercase key".to_string(),
        },
        RawHiveCompanyMember {
            membership_id: "not-a-uuid".to_string(),
            public_key: Some("c".repeat(64)),
            display_name: "in\nvalid".to_string(),
        },
    ]);

    assert_eq!(members.len(), 1);
    assert_eq!(members[0].display_name, "Andrew");
    assert_eq!(members[0].public_key, "a".repeat(64));
    assert_eq!(
        members[0].membership_id,
        "10000000-0000-4000-8000-000000000001"
    );
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

#[test]
fn public_company_member_projection_contains_only_opaque_membership_selector() {
    let member = HiveCompanyMember {
        membership_id: "10000000-0000-4000-8000-000000000001".to_string(),
        public_key: "a".repeat(64),
        display_name: "Andrew".to_string(),
    };
    let json = serde_json::to_string(&member).unwrap();
    assert!(!json.contains("desktop_session"));
    assert!(json.contains("membershipId"));
    assert!(!json.contains("room"));
    assert!(!json.contains("email"));
    assert!(!json.contains("nsec"));
}
