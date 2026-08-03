use super::keychain_migration::{
    active_session_entries, pending_identity_rotation_key, select_legacy_identity_candidate,
    staged_identity_rotation_entries,
};
#[cfg(feature = "evaos-teams-managed")]
use super::login_identity::require_genuine_native_identity_loss;
use super::*;
use nostr::ToBech32;

#[test]
fn managed_session_keyring_service_remains_upgrade_compatible() {
    assert_eq!(KEYRING_SERVICE, "evaos-teams-desktop");
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

fn authoritative_binding(public_key: Option<String>) -> IdentityBinding {
    IdentityBinding {
        membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key,
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

#[tokio::test]
async fn manual_backup_code_completes_only_the_current_pending_login() {
    let state = EvaosTeamsState::default();
    let (sender, receiver) = oneshot::channel();
    let callback = std::sync::Arc::new(LoginCallback {
        expected_state: "state-12345678".to_string(),
        sender: Mutex::new(Some(sender)),
    });
    let registration = register_pending_login(&state, callback).unwrap();

    callback::submit_pending_login_code(&state, "aabb-ccdd-eeff-0011").unwrap();
    assert_eq!(receiver.await.unwrap().unwrap(), "AABBCCDDEEFF0011");
    assert!(callback::submit_pending_login_code(&state, "aabb-ccdd-eeff-0011").is_err());

    drop(registration);
    assert!(callback::submit_pending_login_code(&state, "aabb-ccdd-eeff-0011").is_err());
}

#[test]
fn manual_backup_code_rejects_invalid_or_unpaired_input() {
    let state = EvaosTeamsState::default();
    assert!(callback::submit_pending_login_code(&state, "short").is_err());
    assert!(callback::submit_pending_login_code(&state, "aabb-ccdd-eeff-0011").is_err());
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
fn identity_binding_requires_authoritative_scope_fields() {
    let response = serde_json::json!({
        "status": "active",
        "binding": {
            "membership_id": "10000000-0000-4000-8000-000000000002",
            "community_id": "10000000-0000-4000-8000-000000000003",
            "relay_host": "https://relay.example.com",
            "public_key": null
        }
    });
    assert!(serde_json::from_value::<IdentityBindingResponse>(response.clone()).is_ok());

    for missing_field in ["community_id", "relay_host"] {
        let mut incomplete = response.clone();
        incomplete["binding"]
            .as_object_mut()
            .unwrap()
            .remove(missing_field);
        assert!(serde_json::from_value::<IdentityBindingResponse>(incomplete).is_err());
    }
}

#[test]
fn identity_binding_validator_requires_every_authoritative_field() {
    let public_key = Keys::generate().public_key().to_hex();
    let binding = IdentityBinding {
        membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: Some(public_key.clone()),
    };
    assert!(identity_binding::validate_identity_binding(
        &binding,
        &binding.membership_id,
        &binding.community_id,
        &binding.relay_host,
        &public_key,
    )
    .is_ok());

    let mismatches = [
        (
            "20000000-0000-4000-8000-000000000002",
            binding.community_id.as_str(),
            binding.relay_host.as_str(),
            public_key.as_str(),
        ),
        (
            binding.membership_id.as_str(),
            "20000000-0000-4000-8000-000000000003",
            binding.relay_host.as_str(),
            public_key.as_str(),
        ),
        (
            binding.membership_id.as_str(),
            binding.community_id.as_str(),
            "https://other.example.com",
            public_key.as_str(),
        ),
        (
            binding.membership_id.as_str(),
            binding.community_id.as_str(),
            binding.relay_host.as_str(),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
    ];
    for (membership_id, community_id, relay_host, expected_public_key) in mismatches {
        assert!(identity_binding::validate_identity_binding(
            &binding,
            membership_id,
            community_id,
            relay_host,
            expected_public_key,
        )
        .is_err());
    }
}

#[test]
fn challenge_signature_uses_exact_server_template_and_rejects_tampering() {
    let keys = Keys::generate();
    let response = challenge(&keys);
    let binding = authoritative_binding(None);
    let event = signed_challenge(&response, &keys, &binding).unwrap();
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
    assert!(signed_challenge(&altered, &keys, &binding).is_err());

    let mut wrong_membership = authoritative_binding(None);
    wrong_membership.membership_id = "20000000-0000-4000-8000-000000000002".to_string();
    assert!(signed_challenge(&response, &keys, &wrong_membership).is_err());
}

#[test]
fn ordinary_challenge_signing_rejects_authoritative_community_or_relay_mismatch() {
    let keys = Keys::generate();
    let response = challenge(&keys);

    let mut wrong_community = authoritative_binding(None);
    wrong_community.community_id = "20000000-0000-4000-8000-000000000003".to_string();
    assert!(signed_challenge(&response, &keys, &wrong_community).is_err());

    let mut wrong_relay = authoritative_binding(None);
    wrong_relay.relay_host = "https://other.example.com".to_string();
    assert!(signed_challenge(&response, &keys, &wrong_relay).is_err());
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
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: Some(keys.public_key().to_hex()),
    };
    assert!(verify_existing_native_identity(&binding, &keys).unwrap());

    let state = crate::app_state::build_app_state();
    *state.keys.lock().unwrap() = keys.clone();
    install_entitlement(
        &state,
        &keys,
        &binding,
        &binding.membership_id,
        &entitlement(Some(keys.public_key().to_hex())),
    )
    .unwrap();

    assert_eq!(state.keys.lock().unwrap().public_key(), keys.public_key());
    assert_eq!(
        state.relay_url_override.lock().unwrap().as_deref(),
        Some("wss://relay.example.com")
    );
    assert!(
        state
            .managed_entitlement_expires_at_unix
            .load(std::sync::atomic::Ordering::Acquire)
            > chrono::Utc::now().timestamp()
    );
}

#[test]
fn entitlement_install_rejects_a_stale_verification_identity() {
    let verified_keys = Keys::generate();
    let state = crate::app_state::build_app_state();
    *state.keys.lock().unwrap() = Keys::generate();
    let binding = IdentityBinding {
        membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: Some(verified_keys.public_key().to_hex()),
    };

    let result = install_entitlement(
        &state,
        &verified_keys,
        &binding,
        &binding.membership_id,
        &entitlement(Some(verified_keys.public_key().to_hex())),
    );

    assert!(result.is_err());
    assert!(state.relay_url_override.lock().unwrap().is_none());
    assert_eq!(
        state
            .managed_entitlement_expires_at_unix
            .load(std::sync::atomic::Ordering::Acquire),
        0
    );
}

#[test]
fn entitlement_install_rejects_authoritative_scope_mismatch() {
    let keys = Keys::generate();
    let state = crate::app_state::build_app_state();
    *state.keys.lock().unwrap() = keys.clone();
    let binding = IdentityBinding {
        membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: Some(keys.public_key().to_hex()),
    };

    for mismatched in [
        EvaosTeamsEntitlement {
            community_id: "20000000-0000-4000-8000-000000000003".to_string(),
            ..entitlement(Some(keys.public_key().to_hex()))
        },
        EvaosTeamsEntitlement {
            relay_host: "https://other.example.com".to_string(),
            ..entitlement(Some(keys.public_key().to_hex()))
        },
    ] {
        assert!(
            install_entitlement(&state, &keys, &binding, &binding.membership_id, &mismatched,)
                .is_err()
        );
        assert!(state.relay_url_override.lock().unwrap().is_none());
    }
}

#[test]
fn unbound_membership_enrolls_but_mismatched_canonical_key_requires_recovery() {
    let keys = Keys::generate();
    let state = crate::app_state::build_app_state();
    *state.keys.lock().unwrap() = keys.clone();
    let before_key = state.keys.lock().unwrap().public_key();
    let before_relay = state.relay_url_override.lock().unwrap().clone();

    let unbound = IdentityBinding {
        membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: None,
    };
    assert!(!verify_existing_native_identity(&unbound, &keys).unwrap());

    let mismatched = IdentityBinding {
        membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: Some(Keys::generate().public_key().to_hex()),
    };
    let error = verify_existing_native_identity(&mismatched, &keys).unwrap_err();
    let status = EvaosTeamsAuthStatus::managed("reauth_required", Some(error));
    assert_eq!(status.phase, "reauth_required");
    assert!(!status.authenticated);

    assert_eq!(state.keys.lock().unwrap().public_key(), before_key);
    assert_eq!(*state.relay_url_override.lock().unwrap(), before_relay);
}

#[test]
fn managed_store_shape_preserves_known_legacy_identity_entries() {
    let session_only =
        HashMap::from([(SESSION_KEY.to_string(), "opaque-session-value".to_string())]);
    let runtime = runtime_from_entries(Some(session_only)).unwrap();
    assert!(runtime.session.is_some());
    assert!(!runtime.logout_pending);

    let legacy = HashMap::from([
        (SESSION_KEY.to_string(), "opaque-session-value".to_string()),
        (LOGOUT_PENDING_KEY.to_string(), "1".to_string()),
        ("identity".to_string(), "nsec1forbidden".to_string()),
        (
            "active_membership_id".to_string(),
            "10000000-0000-4000-8000-000000000002".to_string(),
        ),
        (
            "identity:10000000-0000-4000-8000-000000000002".to_string(),
            "nsec1forbidden".to_string(),
        ),
    ]);
    let validated = validated_runtime_entries(Some(legacy.clone())).unwrap();
    assert_eq!(validated, legacy);
    let runtime = runtime_from_entries(Some(validated)).unwrap();
    assert!(runtime.session.is_some());
    assert!(runtime.logout_pending);
}

#[test]
fn pending_session_preserves_only_legacy_identity_material() {
    let keys = Keys::generate();
    let membership_id = "10000000-0000-4000-8000-000000000002";
    let mut stored = HashMap::from([
        (
            format!("{LEGACY_IDENTITY_KEY_PREFIX}{membership_id}"),
            keys.secret_key().to_bech32().unwrap(),
        ),
        (
            LEGACY_ACTIVE_MEMBERSHIP_KEY.to_string(),
            membership_id.to_string(),
        ),
    ]);
    let mut pending = preserve_legacy_identity_entries(&stored).unwrap();
    pending.extend(pending_session_entries("new-session"));

    assert_eq!(
        pending.get(&format!("{LEGACY_IDENTITY_KEY_PREFIX}{membership_id}")),
        stored.get(&format!("{LEGACY_IDENTITY_KEY_PREFIX}{membership_id}"))
    );
    assert_eq!(
        pending.get(LEGACY_ACTIVE_MEMBERSHIP_KEY),
        Some(&membership_id.to_string())
    );
    assert_eq!(
        pending.get(SESSION_KEY).map(String::as_str),
        Some("new-session")
    );
    assert_eq!(
        pending.get(LOGOUT_PENDING_KEY).map(String::as_str),
        Some("1")
    );

    stored.insert(SESSION_KEY.to_string(), "old-session".to_string());
    let signed_out = preserve_legacy_identity_entries(&stored).unwrap();
    assert!(!signed_out.contains_key(SESSION_KEY));
    assert!(signed_out.contains_key(&format!("{LEGACY_IDENTITY_KEY_PREFIX}{membership_id}")));
}

#[test]
fn staged_identity_rotation_is_membership_scoped_durable_and_cleared_on_adoption() {
    let membership_id = "10000000-0000-4000-8000-000000000002";
    let other_membership_id = "10000000-0000-4000-8000-000000000003";
    let (first, keys) = staged_identity_rotation_entries(&HashMap::new(), membership_id).unwrap();
    let (second, reused) = staged_identity_rotation_entries(&first, membership_id).unwrap();
    let (with_other, other) =
        staged_identity_rotation_entries(&second, other_membership_id).unwrap();

    assert_eq!(keys.public_key(), reused.public_key());
    assert_ne!(keys.public_key(), other.public_key());
    assert!(with_other.contains_key(&pending_identity_rotation_key(membership_id).unwrap()));
    assert!(with_other.contains_key(&pending_identity_rotation_key(other_membership_id).unwrap()));

    let active = active_session_entries(
        &with_other,
        "active-session",
        membership_id,
        &keys.public_key().to_hex(),
    )
    .unwrap();
    assert!(!active.contains_key(&pending_identity_rotation_key(membership_id).unwrap()));
    assert!(active.contains_key(&pending_identity_rotation_key(other_membership_id).unwrap()));
}

#[test]
fn identity_reset_status_exposes_no_credential_or_identity_material() {
    let status = EvaosTeamsAuthStatus::managed(
        "identity_reset_required",
        Some("The prior Hive key is unavailable".to_string()),
    );
    let serialized = serde_json::to_string(&status).unwrap();
    assert!(serialized.contains("identity_reset_required"));
    for forbidden in ["session", "membership", "publicKey", "nsec", "private"] {
        assert!(!serialized.contains(forbidden));
    }
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn identity_reset_requires_genuine_native_identity_loss() {
    assert!(require_genuine_native_identity_loss(true, false).is_ok());
    assert!(require_genuine_native_identity_loss(false, true).is_err());
    assert!(require_genuine_native_identity_loss(false, false).is_err());
    assert!(require_genuine_native_identity_loss(true, true).is_err());
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn healthy_mismatched_native_identity_recovers_on_login_but_reauths_on_resume() {
    let local_keys = Keys::generate();
    let canonical_keys = Keys::generate();
    let binding = authoritative_binding(Some(canonical_keys.public_key().to_hex()));

    // Managed sign-in: a readable non-canonical key is "not ready", so
    // complete_login falls through to managed recovery instead of erroring.
    assert!(!login_identity::local_identity_ready_for_login(&binding, Some(&local_keys)).unwrap());

    // Session resume cannot recover in place, so it still hard-errors and
    // defers to a fresh sign-in rather than rebinding the local key.
    assert!(verify_existing_native_identity(&binding, &local_keys).is_err());

    // A present, unlocked identity is never treated as identity loss.
    assert!(require_genuine_native_identity_loss(false, false).is_err());
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn matching_native_identity_is_ready_for_login() {
    let keys = Keys::generate();
    let binding = authoritative_binding(Some(keys.public_key().to_hex()));

    assert!(login_identity::local_identity_ready_for_login(&binding, Some(&keys)).unwrap());
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn login_identity_validation_failures_remain_hard_errors() {
    let keys = Keys::generate();

    let mut malformed_membership = authoritative_binding(Some(keys.public_key().to_hex()));
    malformed_membership.membership_id = "not-a-uuid".to_string();
    assert!(
        login_identity::local_identity_ready_for_login(&malformed_membership, Some(&keys)).is_err()
    );

    let invalid_canonical = authoritative_binding(Some("z".repeat(64)));
    assert!(
        login_identity::local_identity_ready_for_login(&invalid_canonical, Some(&keys)).is_err()
    );
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn genuine_identity_loss_remains_reset_eligible_without_local_keys() {
    let binding = authoritative_binding(Some(Keys::generate().public_key().to_hex()));

    assert!(require_genuine_native_identity_loss(true, false).is_ok());
    assert!(!login_identity::local_identity_ready_for_login(&binding, None).unwrap());
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn unbound_membership_is_not_ready_for_login() {
    let keys = Keys::generate();
    let binding = authoritative_binding(None);

    assert!(!login_identity::local_identity_ready_for_login(&binding, Some(&keys)).unwrap());
}

#[test]
fn legacy_identity_candidate_requires_canonical_membership_match() {
    let matching = Keys::generate();
    let other = Keys::generate();
    let membership_id = "10000000-0000-4000-8000-000000000002";
    let other_membership_id = "10000000-0000-4000-8000-000000000003";
    let stored = HashMap::from([
        (
            format!("{LEGACY_IDENTITY_KEY_PREFIX}{membership_id}"),
            matching.secret_key().to_bech32().unwrap(),
        ),
        (
            format!("{LEGACY_IDENTITY_KEY_PREFIX}{other_membership_id}"),
            other.secret_key().to_bech32().unwrap(),
        ),
    ]);

    let candidate =
        select_legacy_identity_candidate(&stored, membership_id, &matching.public_key().to_hex())
            .unwrap()
            .unwrap();
    assert_eq!(candidate.public_key(), matching.public_key());
    assert!(
        select_legacy_identity_candidate(&stored, membership_id, &other.public_key().to_hex())
            .unwrap()
            .is_none()
    );
}

#[test]
fn legacy_identity_candidate_deduplicates_same_unscoped_key() {
    let keys = Keys::generate();
    let membership_id = "10000000-0000-4000-8000-000000000002";
    let encoded = keys.secret_key().to_bech32().unwrap();
    let stored = HashMap::from([
        (
            format!("{LEGACY_IDENTITY_KEY_PREFIX}{membership_id}"),
            encoded.clone(),
        ),
        (LEGACY_IDENTITY_KEY.to_string(), encoded),
    ]);

    let candidate =
        select_legacy_identity_candidate(&stored, membership_id, &keys.public_key().to_hex())
            .unwrap()
            .unwrap();
    assert_eq!(candidate.public_key(), keys.public_key());
}

#[test]
fn malformed_legacy_identity_fails_without_normalizing_it_away() {
    let membership_id = "10000000-0000-4000-8000-000000000002";
    let stored = HashMap::from([(
        format!("{LEGACY_IDENTITY_KEY_PREFIX}{membership_id}"),
        "not-a-private-key".to_string(),
    )]);

    assert!(select_legacy_identity_candidate(
        &stored,
        membership_id,
        &Keys::generate().public_key().to_hex(),
    )
    .unwrap()
    .is_none());
    assert_eq!(
        validated_runtime_entries(Some(stored.clone())).unwrap(),
        stored
    );
}

#[test]
fn active_session_removes_only_the_adopted_legacy_identity() {
    let adopted = Keys::generate();
    let other = Keys::generate();
    let membership_id = "10000000-0000-4000-8000-000000000002";
    let other_membership_id = "10000000-0000-4000-8000-000000000003";
    let adopted_key = format!("{LEGACY_IDENTITY_KEY_PREFIX}{membership_id}");
    let other_key = format!("{LEGACY_IDENTITY_KEY_PREFIX}{other_membership_id}");
    let stored = HashMap::from([
        (
            adopted_key.clone(),
            adopted.secret_key().to_bech32().unwrap(),
        ),
        (other_key.clone(), other.secret_key().to_bech32().unwrap()),
        (
            LEGACY_IDENTITY_KEY.to_string(),
            "malformed-preserved-value".to_string(),
        ),
        (
            LEGACY_ACTIVE_MEMBERSHIP_KEY.to_string(),
            membership_id.to_string(),
        ),
        (LOGOUT_PENDING_KEY.to_string(), "1".to_string()),
    ]);

    let replacement = active_session_entries(
        &stored,
        "active-session",
        membership_id,
        &adopted.public_key().to_hex(),
    )
    .unwrap();

    assert!(!replacement.contains_key(&adopted_key));
    assert_eq!(replacement.get(&other_key), stored.get(&other_key));
    assert_eq!(
        replacement.get(LEGACY_IDENTITY_KEY),
        Some(&"malformed-preserved-value".to_string())
    );
    assert!(!replacement.contains_key(LEGACY_ACTIVE_MEMBERSHIP_KEY));
    assert!(!replacement.contains_key(LOGOUT_PENDING_KEY));
    assert_eq!(
        replacement.get(SESSION_KEY).map(String::as_str),
        Some("active-session")
    );
}

#[test]
fn managed_store_shape_still_rejects_unknown_material() {
    for unknown_key in ["unexpected_secret", "identity:not-a-membership-id"] {
        let forbidden = HashMap::from([
            (SESSION_KEY.to_string(), "opaque-session-value".to_string()),
            (unknown_key.to_string(), "value".to_string()),
        ]);
        assert!(runtime_from_entries(Some(forbidden)).is_err());
    }
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn entitlement_binding_requires_the_verified_public_key() {
    let keys = Keys::generate();
    let binding = IdentityBinding {
        membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: Some(keys.public_key().to_hex()),
    };
    let verified_entitlement = entitlement(Some(keys.public_key().to_hex()));

    assert!(login_identity::binding_for_entitlement(&binding, &verified_entitlement).is_ok());
    assert!(login_identity::binding_for_entitlement(&binding, &entitlement(None)).is_err());
    assert!(login_identity::binding_for_entitlement(
        &binding,
        &EvaosTeamsEntitlement {
            community_id: "20000000-0000-4000-8000-000000000003".to_string(),
            ..verified_entitlement.clone()
        },
    )
    .is_err());
    assert!(login_identity::binding_for_entitlement(
        &binding,
        &EvaosTeamsEntitlement {
            relay_host: "https://other.example.com".to_string(),
            ..verified_entitlement
        },
    )
    .is_err());
}

#[test]
fn newly_claimed_session_remains_logout_pending_until_login_commits() {
    let runtime =
        runtime_from_entries(Some(pending_session_entries("opaque-session-value"))).unwrap();

    assert!(runtime.session.is_some());
    assert!(runtime.logout_pending);
    assert!(!runtime.custody_checked);
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn failed_explicit_logout_stays_in_retry_mode_and_disables_identity_reset() {
    let state = EvaosTeamsState::default();
    *state.runtime.lock().unwrap() = ManagedRuntime {
        initialized: true,
        session: Some(Zeroizing::new("opaque-session-value".to_string())),
        logout_pending: false,
        custody_checked: false,
    };
    *state.pending_identity_reset.lock().unwrap() = Some(PendingIdentityReset {
        session: Zeroizing::new("opaque-session-value".to_string()),
        membership_id: "10000000-0000-4000-8000-000000000002".to_string(),
        community_id: "10000000-0000-4000-8000-000000000003".to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: "a".repeat(64),
    });

    clear_pending_identity_reset_for_logout(&state).unwrap();
    state.runtime.lock().unwrap().logout_pending = true;

    let (_, logout_pending) = current_session(&state).unwrap();
    assert!(logout_pending);
    assert!(login_identity::pending_identity_reset_status(&state)
        .unwrap()
        .is_none());
    assert!(state.pending_identity_reset.lock().unwrap().is_none());

    let restarted =
        runtime_from_entries(Some(pending_session_entries("opaque-session-value"))).unwrap();
    assert!(restarted.session.is_some());
    assert!(restarted.logout_pending);
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn reauthentication_revokes_only_a_distinct_previous_session() {
    assert_eq!(
        previous_session_to_revoke(Some("previous-session"), "claimed-session"),
        Some("previous-session".to_string())
    );
    assert_eq!(
        previous_session_to_revoke(Some("claimed-session"), "claimed-session"),
        None
    );
    assert_eq!(previous_session_to_revoke(None, "claimed-session"), None);
}

#[test]
fn public_status_never_serializes_backend_proof_or_credentials() {
    let status = EvaosTeamsAuthStatus::managed(
        "reauth_required",
        Some("Sign in again to recover this device".to_string()),
    );
    let json = serde_json::to_string(&status).unwrap();
    for forbidden in [
        "desktop_session",
        "nsec",
        "device_code",
        "verifier",
        "challenge",
        "private_key",
        "payload_ciphertext",
        "sealed_data_key",
        "wrapped_data_key",
    ] {
        assert!(!json.contains(forbidden), "{forbidden}");
    }
}
