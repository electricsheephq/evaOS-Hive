use super::*;

fn fixture_ids() -> (&'static str, &'static str, &'static str) {
    (
        "10000000-0000-4000-8000-000000000001",
        "10000000-0000-4000-8000-000000000002",
        "10000000-0000-4000-8000-000000000003",
    )
}

fn fixture_expiry() -> String {
    (chrono::Utc::now() + chrono::Duration::minutes(1)).to_rfc3339()
}

#[test]
fn only_exact_missing_envelope_response_offers_identity_reset() {
    assert!(matches!(
        classify_recovery_issue_error(ApiFailure {
            status: reqwest::StatusCode::NOT_FOUND,
            code: "identity_recovery_not_available".to_string(),
        }),
        IdentityRecoveryError::NotAvailable
    ));
    for error in [
        ApiFailure {
            status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
            code: "network_error:true".to_string(),
        },
        ApiFailure {
            status: reqwest::StatusCode::NOT_FOUND,
            code: "other_missing_resource".to_string(),
        },
        ApiFailure {
            status: reqwest::StatusCode::FORBIDDEN,
            code: "identity_recovery_not_available".to_string(),
        },
    ] {
        assert!(matches!(
            classify_recovery_issue_error(error),
            IdentityRecoveryError::Other(_)
        ));
    }
}

fn fixture_challenge(keys: &Keys) -> ChallengeResponse {
    let (identity_id, membership_id, community_id) = fixture_ids();
    ChallengeResponse {
        status: "challenge_issued".to_string(),
        challenge: super::super::KeyBindingChallenge {
            schema_version: super::super::KEY_BINDING_SCHEMA.to_string(),
            identity_id: identity_id.to_string(),
            membership_id: membership_id.to_string(),
            community_id: community_id.to_string(),
            desktop_session_id: "10000000-0000-4000-8000-000000000004".to_string(),
            public_key: keys.public_key().to_hex(),
            nonce: encode_base64url(&[1_u8; 32]),
            expires_at: fixture_expiry(),
        },
        event_template: super::super::EventTemplate {
            kind: super::super::KEY_BINDING_KIND,
            created_at: Timestamp::now().as_secs(),
            tags: Vec::new(),
            content: String::new(),
        },
        relay_host: "https://relay.example.com".to_string(),
    }
}

fn fixture_recovery(keys: &Keys) -> RecoveryChallenge {
    RecoveryChallenge {
        recovery_id: "10000000-0000-4000-8000-000000000005".to_string(),
        public_key: keys.public_key().to_hex(),
        key_epoch: 1,
        access_revision: 7,
        expires_at: fixture_expiry(),
        device_public_key_sha256: "0".repeat(64),
        sealed_challenge: DeviceSealedValue {
            schema_version: SEALED_VALUE_SCHEMA.to_string(),
            suite: DEVICE_HPKE_SUITE.to_string(),
            encapsulated_key: encode_base64url(&[4_u8; 65]),
            ciphertext: encode_base64url(&[0_u8; 48]),
        },
    }
}

fn fixture_payload(keys: &Keys) -> RecoveryPayload {
    let (identity_id, _, _) = fixture_ids();
    RecoveryPayload {
        schema_version: RECOVERY_SCHEMA.to_string(),
        identity_id: identity_id.to_string(),
        public_key: keys.public_key().to_hex(),
        key_epoch: 1,
        access_revision: 7,
        payload_ciphertext: encode_base64url(&[0_u8; 48]),
        payload_nonce: encode_base64url(&[0_u8; 12]),
        payload_sha256: "0".repeat(64),
        sealed_data_key: DeviceSealedValue {
            schema_version: SEALED_VALUE_SCHEMA.to_string(),
            suite: DEVICE_HPKE_SUITE.to_string(),
            encapsulated_key: encode_base64url(&[4_u8; 65]),
            ciphertext: encode_base64url(&[0_u8; 48]),
        },
        next_action: "verify_key_challenge".to_string(),
    }
}

#[test]
fn custody_context_is_byte_exact_and_stable() {
    let context = custody_context(
        "10000000-0000-4000-8000-000000000001",
        "10000000-0000-4000-8000-000000000002",
        "10000000-0000-4000-8000-000000000003",
    )
    .unwrap();
    assert_eq!(
        context,
        concat!(
            "{\"application\":\"evaos-hive\",",
            "\"community_id\":\"10000000-0000-4000-8000-000000000003\",",
            "\"identity_id\":\"10000000-0000-4000-8000-000000000001\",",
            "\"membership_id\":\"10000000-0000-4000-8000-000000000002\",",
            "\"schema\":\"evaos.hive_identity_envelope.v1\"}"
        )
    );
}

#[test]
fn envelope_round_trip_preserves_exact_native_identity() {
    let keys = Keys::generate();
    let data_key = Zeroizing::new([7_u8; 32]);
    let context = custody_context(
        "10000000-0000-4000-8000-000000000001",
        "10000000-0000-4000-8000-000000000002",
        "10000000-0000-4000-8000-000000000003",
    )
    .unwrap();
    let (payload_ciphertext, payload_nonce, payload_sha256) =
        encrypt_secret_key(&keys, &*data_key, &context).unwrap();
    let payload = RecoveryPayload {
        schema_version: RECOVERY_SCHEMA.to_string(),
        identity_id: "10000000-0000-4000-8000-000000000001".to_string(),
        public_key: keys.public_key().to_hex(),
        key_epoch: 1,
        access_revision: 7,
        payload_ciphertext,
        payload_nonce,
        payload_sha256,
        sealed_data_key: DeviceSealedValue {
            schema_version: SEALED_VALUE_SCHEMA.to_string(),
            suite: DEVICE_HPKE_SUITE.to_string(),
            encapsulated_key: encode_base64url(&[4_u8; 65]),
            ciphertext: encode_base64url(&[0_u8; 48]),
        },
        next_action: "verify_key_challenge".to_string(),
    };
    let recovered = decrypt_secret_key(&payload, &*data_key, &context).unwrap();
    assert_eq!(recovered.public_key(), keys.public_key());
}

#[test]
fn envelope_decryption_fails_for_wrong_context_key_and_digest() {
    let keys = Keys::generate();
    let data_key = Zeroizing::new([9_u8; 32]);
    let context = custody_context(
        "10000000-0000-4000-8000-000000000001",
        "10000000-0000-4000-8000-000000000002",
        "10000000-0000-4000-8000-000000000003",
    )
    .unwrap();
    let (payload_ciphertext, payload_nonce, payload_sha256) =
        encrypt_secret_key(&keys, &*data_key, &context).unwrap();
    let mut payload = RecoveryPayload {
        schema_version: RECOVERY_SCHEMA.to_string(),
        identity_id: "10000000-0000-4000-8000-000000000001".to_string(),
        public_key: keys.public_key().to_hex(),
        key_epoch: 1,
        access_revision: 7,
        payload_ciphertext,
        payload_nonce,
        payload_sha256,
        sealed_data_key: DeviceSealedValue {
            schema_version: SEALED_VALUE_SCHEMA.to_string(),
            suite: DEVICE_HPKE_SUITE.to_string(),
            encapsulated_key: encode_base64url(&[4_u8; 65]),
            ciphertext: encode_base64url(&[0_u8; 48]),
        },
        next_action: "verify_key_challenge".to_string(),
    };
    assert!(decrypt_secret_key(&payload, &*data_key, "wrong-context").is_err());
    assert!(decrypt_secret_key(&payload, &[8_u8; 32], &context).is_err());
    payload.payload_sha256 = "0".repeat(64);
    assert!(decrypt_secret_key(&payload, &*data_key, &context).is_err());
}

#[test]
fn recovery_scope_and_revision_validation_fail_closed() {
    let keys = Keys::generate();
    let (_, membership_id, _) = fixture_ids();
    let binding = IdentityBinding {
        membership_id: membership_id.to_string(),
        public_key: Some(keys.public_key().to_hex()),
    };
    let challenge = fixture_challenge(&keys);
    let recovery = fixture_recovery(&keys);
    let payload = fixture_payload(&keys);
    assert!(validate_recovery_payload(&payload, &recovery, &binding, &challenge).is_ok());

    let entitlement = EvaosTeamsEntitlement {
        community_id: challenge.challenge.community_id.clone(),
        relay_host: challenge.relay_host.clone(),
        public_key: Some(keys.public_key().to_hex()),
        role: "member".to_string(),
        access_revision: recovery.access_revision,
        expires_at: fixture_expiry(),
        refresh_after_seconds: 300,
    };
    assert!(validate_recovered_entitlement(&entitlement, &recovery).is_ok());

    let wrong_membership = IdentityBinding {
        membership_id: "10000000-0000-4000-8000-000000000099".to_string(),
        public_key: binding.public_key.clone(),
    };
    assert!(validate_recovery_payload(&payload, &recovery, &wrong_membership, &challenge).is_err());

    let mut wrong_identity = fixture_payload(&keys);
    wrong_identity.identity_id = "10000000-0000-4000-8000-000000000098".to_string();
    assert!(validate_recovery_payload(&wrong_identity, &recovery, &binding, &challenge).is_err());

    let mut stale_entitlement = entitlement;
    stale_entitlement.access_revision += 1;
    assert!(validate_recovered_entitlement(&stale_entitlement, &recovery).is_err());
}

#[test]
fn recovery_challenge_binds_device_and_rejects_expiry() {
    let keys = Keys::generate();
    let (_, membership_id, _) = fixture_ids();
    let binding = IdentityBinding {
        membership_id: membership_id.to_string(),
        public_key: Some(keys.public_key().to_hex()),
    };
    let device = DeviceTransport::generate().unwrap();
    let mut recovery = fixture_recovery(&keys);
    recovery.device_public_key_sha256 = device_public_key_sha256(&device).unwrap();
    assert!(validate_recovery_challenge(&recovery, &device, &binding).is_ok());

    recovery.expires_at = (chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339();
    assert!(validate_recovery_challenge(&recovery, &device, &binding).is_err());
    recovery.expires_at = fixture_expiry();
    recovery.device_public_key_sha256 = "f".repeat(64);
    assert!(validate_recovery_challenge(&recovery, &device, &binding).is_err());
}

#[test]
fn enrollment_event_is_native_signed_and_canonical() {
    let keys = Keys::generate();
    let (identity_id, membership_id, community_id) = fixture_ids();
    let context = custody_context(identity_id, membership_id, community_id).unwrap();
    let enrollment = EnrollmentChallenge {
        schema_version: ENVELOPE_SCHEMA.to_string(),
        challenge_id: "10000000-0000-4000-8000-000000000006".to_string(),
        identity_id: identity_id.to_string(),
        membership_id: membership_id.to_string(),
        community_id: community_id.to_string(),
        desktop_session_id: "10000000-0000-4000-8000-000000000004".to_string(),
        public_key: keys.public_key().to_hex(),
        access_revision: 7,
        nonce: encode_base64url(&[2_u8; 32]),
        expires_at: fixture_expiry(),
        custody_context_sha256: sha256_hex(context.as_bytes()),
        sealed_data_key: fixture_recovery(&keys).sealed_challenge,
    };
    let binding = IdentityBinding {
        membership_id: membership_id.to_string(),
        public_key: Some(keys.public_key().to_hex()),
    };
    let entitlement = EvaosTeamsEntitlement {
        community_id: community_id.to_string(),
        relay_host: "https://relay.example.com".to_string(),
        public_key: Some(keys.public_key().to_hex()),
        role: "member".to_string(),
        access_revision: 7,
        expires_at: fixture_expiry(),
        refresh_after_seconds: 300,
    };
    assert_eq!(
        validate_enrollment(&enrollment, &binding, &entitlement, &keys).unwrap(),
        context
    );

    let mut wrong_community = entitlement.clone();
    wrong_community.community_id = "10000000-0000-4000-8000-000000000099".to_string();
    assert!(validate_enrollment(&enrollment, &binding, &wrong_community, &keys).is_err());

    let mut stale_revision = entitlement;
    stale_revision.access_revision += 1;
    assert!(validate_enrollment(&enrollment, &binding, &stale_revision, &keys).is_err());

    let payload_sha256 = "a".repeat(64);
    let event = signed_enrollment_event(&enrollment, &payload_sha256, &keys).unwrap();
    assert_eq!(event["kind"], ENVELOPE_KIND);
    assert_eq!(event["pubkey"], keys.public_key().to_hex());
    assert_eq!(
        event["tags"],
        serde_json::json!([
            ["t", "evaos-hive-identity-envelope"],
            ["challenge", enrollment.nonce],
            ["payload", payload_sha256],
        ])
    );
    let content: serde_json::Value =
        serde_json::from_str(event["content"].as_str().unwrap()).unwrap();
    assert_eq!(content["identity_id"], identity_id);
    assert_eq!(content["access_revision"], 7);
}

#[test]
fn hpke_device_transport_round_trip_uses_dashboard_suite() {
    let device = DeviceTransport::generate().unwrap();
    let public_key_bytes = decode_base64url(&device.public_key, 65, 65).unwrap();
    let public_key = <DeviceKem as KemTrait>::PublicKey::from_bytes(&public_key_bytes).unwrap();
    let info = b"evaos.hive.recovery.challenge.v1:10000000-0000-4000-8000-000000000001";
    let (encapped_key, ciphertext) = hpke::single_shot_seal::<DeviceAead, DeviceKdf, DeviceKem>(
        &hpke::OpModeS::Base,
        &public_key,
        info,
        b"challenge",
        &[],
    )
    .unwrap();
    let mut sealed = DeviceSealedValue {
        schema_version: SEALED_VALUE_SCHEMA.to_string(),
        suite: DEVICE_HPKE_SUITE.to_string(),
        encapsulated_key: encode_base64url(encapped_key.to_bytes().as_ref()),
        ciphertext: encode_base64url(&ciphertext),
    };
    assert_eq!(
        &*device
            .open(&sealed, std::str::from_utf8(info).unwrap())
            .unwrap(),
        b"challenge"
    );
    sealed.suite = "unsupported".to_string();
    assert!(device
        .open(&sealed, std::str::from_utf8(info).unwrap())
        .is_err());
    sealed.suite = DEVICE_HPKE_SUITE.to_string();
    sealed.ciphertext = encode_base64url(&[0_u8; 16]);
    assert!(device
        .open(&sealed, std::str::from_utf8(info).unwrap())
        .is_err());
}

#[test]
fn hpke_opens_dashboard_core_1_9_fixture() {
    // Fixture sealed by the merged dashboard contract's @hpke/core 1.9.0
    // implementation to a P-256 recipient whose test-only scalar is 1.
    let mut private_key = vec![0_u8; 32];
    private_key[31] = 1;
    let device = DeviceTransport {
        private_key: Zeroizing::new(private_key),
        public_key: String::new(),
    };
    let sealed = DeviceSealedValue {
        schema_version: SEALED_VALUE_SCHEMA.to_string(),
        suite: DEVICE_HPKE_SUITE.to_string(),
        encapsulated_key:
            "BLwDqSC-5mt0lMoQqAfwAM7oKJvZt3sUXDUBUUseUHxcfDY3jvy6WnrI7J8GRdpJ-I-XHqBC6HfFvp8EefwbuAA"
                .to_string(),
        ciphertext: "hLtc1m2jpzgwESP5E-wWpD-xzm4Whk5SHQ".to_string(),
    };
    let info = "evaos.hive.recovery.challenge.v1:10000000-0000-4000-8000-000000000005";
    assert_eq!(&*device.open(&sealed, info).unwrap(), b"challenge");
}
