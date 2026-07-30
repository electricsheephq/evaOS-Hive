use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hpke::{
    aead::AesGcm256, kdf::HkdfSha256, kem::DhP256HkdfSha256, Deserializable, Kem as KemTrait,
    OpModeR, Serializable,
};
use nostr::{EventBuilder, Keys, Kind, SecretKey, Tag, Timestamp};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Manager;
use zeroize::{Zeroize, Zeroizing};

use super::{
    http_api::{post_json, ApiFailure},
    issue_key_challenge, verify_key_challenge, ChallengeResponse, EvaosTeamsEntitlement,
    IdentityBinding,
};
use crate::app_state::AppState;

const ENVELOPE_SCHEMA: &str = "evaos.hive_identity_envelope.v1";
const RECOVERY_SCHEMA: &str = "evaos.hive_identity_recovery.v1";
const SEALED_VALUE_SCHEMA: &str = "evaos.hive_device_sealed_value.v1";
const DEVICE_HPKE_SUITE: &str = "DHKEM(P-256,HKDF-SHA256)/HKDF-SHA256/AES-256-GCM";
const ENVELOPE_KIND: u16 = 27_235;
const MAX_SEALED_CIPHERTEXT_BYTES: usize = 4_112;
const RECOVERY_CHALLENGE_INFO: &str = "evaos.hive.recovery.challenge.v1";
const RECOVERY_DEK_INFO: &str = "evaos.hive.recovery.dek.v1";
const ENROLLMENT_DEK_INFO: &str = "evaos.hive.enrollment.dek.v1";

type DeviceKem = DhP256HkdfSha256;
type DeviceKdf = HkdfSha256;
type DeviceAead = AesGcm256;
type EnvelopeNonce = Nonce<<Aes256Gcm as AeadCore>::NonceSize>;
pub(super) enum CustodyEnrollment {
    Completed,
    AlreadyPresent,
}

struct DeviceTransport {
    private_key: Zeroizing<Vec<u8>>,
    public_key: String,
}

#[derive(Debug, Deserialize)]
struct DeviceSealedValue {
    schema_version: String,
    suite: String,
    encapsulated_key: String,
    ciphertext: String,
}

#[derive(Debug, Deserialize)]
struct EnrollmentResponse {
    status: String,
    enrollment: EnrollmentChallenge,
}

#[derive(Debug, Deserialize, Serialize)]
struct EnrollmentChallenge {
    schema_version: String,
    challenge_id: String,
    identity_id: String,
    membership_id: String,
    community_id: String,
    desktop_session_id: String,
    public_key: String,
    access_revision: u64,
    nonce: String,
    expires_at: String,
    custody_context_sha256: String,
    #[serde(skip_serializing)]
    sealed_data_key: DeviceSealedValue,
}

#[derive(Debug, Deserialize)]
struct EnrollmentCompletionResponse {
    status: String,
}

#[derive(Debug, Deserialize)]
struct RecoveryChallengeResponse {
    status: String,
    recovery: RecoveryChallenge,
}

#[derive(Debug, Deserialize)]
struct RecoveryChallenge {
    recovery_id: String,
    public_key: String,
    key_epoch: u64,
    access_revision: u64,
    expires_at: String,
    device_public_key_sha256: String,
    sealed_challenge: DeviceSealedValue,
}

#[derive(Debug, Deserialize)]
struct RecoveryCompletionResponse {
    status: String,
    recovery: RecoveryPayload,
}

#[derive(Debug, Deserialize)]
struct RecoveryPayload {
    schema_version: String,
    identity_id: String,
    public_key: String,
    key_epoch: u64,
    access_revision: u64,
    payload_ciphertext: String,
    payload_nonce: String,
    payload_sha256: String,
    sealed_data_key: DeviceSealedValue,
    next_action: String,
}

#[derive(Serialize)]
struct CustodyContext<'a> {
    application: &'static str,
    community_id: &'a str,
    identity_id: &'a str,
    membership_id: &'a str,
    schema: &'static str,
}

#[derive(Serialize)]
struct SignedEnvelopeChallenge<'a> {
    schema_version: &'static str,
    challenge_id: &'a str,
    identity_id: &'a str,
    membership_id: &'a str,
    community_id: &'a str,
    desktop_session_id: &'a str,
    public_key: &'a str,
    access_revision: u64,
    payload_sha256: &'a str,
    custody_context_sha256: &'a str,
    nonce: &'a str,
    expires_at: &'a str,
}

impl DeviceTransport {
    fn generate() -> Result<Self, String> {
        let mut ikm = Zeroizing::new([0_u8; 32]);
        getrandom::fill(&mut *ikm)
            .map_err(|_| "could not create a device recovery key".to_string())?;
        let (private_key, public_key) = DeviceKem::derive_keypair(&*ikm);
        let public_key = encode_base64url(public_key.to_bytes().as_ref());
        if decode_base64url(&public_key, 65, 65)?[0] != 0x04 {
            return Err("device recovery key has an invalid encoding".to_string());
        }
        Ok(Self {
            private_key: Zeroizing::new(private_key.to_bytes().to_vec()),
            public_key,
        })
    }

    fn open(&self, sealed: &DeviceSealedValue, info: &str) -> Result<Zeroizing<Vec<u8>>, String> {
        validate_sealed_value(sealed)?;
        let encapped_key_bytes = decode_base64url(&sealed.encapsulated_key, 65, 65)?;
        let ciphertext = decode_base64url(&sealed.ciphertext, 16, MAX_SEALED_CIPHERTEXT_BYTES)?;
        let encapped_key = <<DeviceKem as KemTrait>::EncappedKey as Deserializable>::from_bytes(
            &encapped_key_bytes,
        )
        .map_err(|_| "device recovery envelope is invalid".to_string())?;
        let private_key =
            <<DeviceKem as KemTrait>::PrivateKey as Deserializable>::from_bytes(&self.private_key)
                .map_err(|_| "device recovery key is invalid".to_string())?;
        let plaintext = hpke::single_shot_open::<DeviceAead, DeviceKdf, DeviceKem>(
            &OpModeR::Base,
            &private_key,
            &encapped_key,
            info.as_bytes(),
            &ciphertext,
            &[],
        )
        .map_err(|_| "device recovery envelope could not be opened".to_string())?;
        Ok(Zeroizing::new(plaintext))
    }
}

fn encode_base64url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn decode_base64url(value: &str, minimum: usize, maximum: usize) -> Result<Vec<u8>, String> {
    if value.is_empty()
        || value.len() > maximum.saturating_mul(4).saturating_div(3) + 4
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err("managed identity envelope contains invalid base64url".to_string());
    }
    let decoded = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| "managed identity envelope contains invalid base64url".to_string())?;
    if decoded.len() < minimum || decoded.len() > maximum {
        return Err("managed identity envelope has an invalid length".to_string());
    }
    Ok(decoded)
}

fn validate_uuid(value: &str, label: &str) -> Result<(), String> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| format!("managed identity {label} is invalid"))
}

fn validate_hex_64(value: &str, label: &str) -> Result<(), String> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(format!("managed identity {label} is invalid"))
    }
}

fn validate_expiry(value: &str) -> Result<(), String> {
    let expires_at = chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|_| "managed identity challenge expiry is invalid".to_string())?;
    let now = chrono::Utc::now();
    if expires_at <= now || expires_at > now + chrono::Duration::minutes(5) {
        return Err("managed identity challenge has expired".to_string());
    }
    Ok(())
}

fn validate_sealed_value(value: &DeviceSealedValue) -> Result<(), String> {
    if value.schema_version != SEALED_VALUE_SCHEMA || value.suite != DEVICE_HPKE_SUITE {
        return Err("managed identity envelope uses an unsupported cipher suite".to_string());
    }
    Ok(())
}

fn device_public_key_sha256(device: &DeviceTransport) -> Result<String, String> {
    let bytes = decode_base64url(&device.public_key, 65, 65)?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn custody_context(
    identity_id: &str,
    membership_id: &str,
    community_id: &str,
) -> Result<String, String> {
    validate_uuid(identity_id, "identity")?;
    validate_uuid(membership_id, "membership")?;
    validate_uuid(community_id, "community")?;
    serde_json::to_string(&CustodyContext {
        application: "evaos-hive",
        community_id,
        identity_id,
        membership_id,
        schema: ENVELOPE_SCHEMA,
    })
    .map_err(|_| "managed identity custody context could not be encoded".to_string())
}

fn sha256_hex(bytes: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(bytes.as_ref()))
}

fn encrypt_secret_key(
    keys: &Keys,
    data_key: &[u8],
    context: &str,
) -> Result<(String, String, String), String> {
    if data_key.len() != 32 {
        return Err("managed identity data key has an invalid length".to_string());
    }
    let cipher = Aes256Gcm::new_from_slice(data_key)
        .map_err(|_| "managed identity data key is invalid".to_string())?;
    let mut nonce = Zeroizing::new([0_u8; 12]);
    getrandom::fill(&mut *nonce)
        .map_err(|_| "could not create an identity envelope nonce".to_string())?;
    let mut secret = Zeroizing::new(keys.secret_key().to_secret_bytes());
    let nonce_ref = <&EnvelopeNonce>::try_from(&*nonce as &[u8])
        .map_err(|_| "managed identity envelope nonce is invalid".to_string())?;
    let ciphertext = cipher
        .encrypt(
            nonce_ref,
            Payload {
                msg: &*secret,
                aad: context.as_bytes(),
            },
        )
        .map_err(|_| "managed identity envelope encryption failed".to_string())?;
    secret.zeroize();
    let payload_nonce = encode_base64url(&*nonce);
    let payload_ciphertext = encode_base64url(&ciphertext);
    let payload_sha256 = sha256_hex(format!("{payload_nonce}.{payload_ciphertext}").as_bytes());
    Ok((payload_ciphertext, payload_nonce, payload_sha256))
}

fn decrypt_secret_key(
    payload: &RecoveryPayload,
    data_key: &[u8],
    context: &str,
) -> Result<Keys, String> {
    if data_key.len() != 32 {
        return Err("managed identity data key has an invalid length".to_string());
    }
    let expected_payload_sha256 =
        sha256_hex(format!("{}.{}", payload.payload_nonce, payload.payload_ciphertext).as_bytes());
    if expected_payload_sha256 != payload.payload_sha256 {
        return Err("managed identity envelope digest does not match".to_string());
    }
    let nonce = decode_base64url(&payload.payload_nonce, 12, 12)?;
    let ciphertext = decode_base64url(&payload.payload_ciphertext, 48, 1_536)?;
    let cipher = Aes256Gcm::new_from_slice(data_key)
        .map_err(|_| "managed identity data key is invalid".to_string())?;
    let nonce_ref = <&EnvelopeNonce>::try_from(nonce.as_slice())
        .map_err(|_| "managed identity envelope nonce is invalid".to_string())?;
    let plaintext = cipher
        .decrypt(
            nonce_ref,
            Payload {
                msg: &ciphertext,
                aad: context.as_bytes(),
            },
        )
        .map_err(|_| "managed identity envelope decryption failed".to_string())?;
    let plaintext = Zeroizing::new(plaintext);
    if plaintext.len() != SecretKey::LEN {
        return Err("managed identity envelope contains an invalid key".to_string());
    }
    let secret_key = SecretKey::from_slice(&plaintext)
        .map_err(|_| "managed identity envelope contains an invalid key".to_string())?;
    Ok(Keys::new(secret_key))
}

fn signed_enrollment_event(
    enrollment: &EnrollmentChallenge,
    payload_sha256: &str,
    keys: &Keys,
) -> Result<serde_json::Value, String> {
    validate_hex_64(payload_sha256, "payload digest")?;
    let challenge = SignedEnvelopeChallenge {
        schema_version: ENVELOPE_SCHEMA,
        challenge_id: &enrollment.challenge_id,
        identity_id: &enrollment.identity_id,
        membership_id: &enrollment.membership_id,
        community_id: &enrollment.community_id,
        desktop_session_id: &enrollment.desktop_session_id,
        public_key: &enrollment.public_key,
        access_revision: enrollment.access_revision,
        payload_sha256,
        custody_context_sha256: &enrollment.custody_context_sha256,
        nonce: &enrollment.nonce,
        expires_at: &enrollment.expires_at,
    };
    let content = serde_json::to_string(&challenge)
        .map_err(|_| "managed identity enrollment could not be encoded".to_string())?;
    let tags = [
        vec!["t".to_string(), "evaos-hive-identity-envelope".to_string()],
        vec!["challenge".to_string(), enrollment.nonce.clone()],
        vec!["payload".to_string(), payload_sha256.to_string()],
    ]
    .into_iter()
    .map(|tag| {
        Tag::parse(tag).map_err(|_| "managed identity enrollment tag is invalid".to_string())
    })
    .collect::<Result<Vec<_>, _>>()?;
    let event = EventBuilder::new(Kind::Custom(ENVELOPE_KIND), content)
        .tags(tags)
        .custom_created_at(Timestamp::now())
        .sign_with_keys(keys)
        .map_err(|_| "managed identity enrollment could not be signed".to_string())?;
    serde_json::to_value(event)
        .map_err(|_| "managed identity enrollment could not be encoded".to_string())
}

fn validate_enrollment(
    enrollment: &EnrollmentChallenge,
    binding: &IdentityBinding,
    keys: &Keys,
) -> Result<String, String> {
    if enrollment.schema_version != ENVELOPE_SCHEMA
        || enrollment.membership_id != binding.membership_id
        || binding.public_key.as_deref() != Some(enrollment.public_key.as_str())
        || enrollment.public_key != keys.public_key().to_hex()
        || enrollment.access_revision == 0
    {
        return Err("managed identity enrollment changed the server-selected identity".to_string());
    }
    for (value, label) in [
        (&enrollment.challenge_id, "challenge"),
        (&enrollment.identity_id, "identity"),
        (&enrollment.membership_id, "membership"),
        (&enrollment.community_id, "community"),
        (&enrollment.desktop_session_id, "session"),
    ] {
        validate_uuid(value, label)?;
    }
    validate_hex_64(&enrollment.public_key, "public key")?;
    validate_hex_64(&enrollment.custody_context_sha256, "custody context")?;
    if enrollment.nonce.len() != 43
        || !enrollment
            .nonce
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err("managed identity enrollment nonce is invalid".to_string());
    }
    validate_expiry(&enrollment.expires_at)?;
    let context = custody_context(
        &enrollment.identity_id,
        &enrollment.membership_id,
        &enrollment.community_id,
    )?;
    if sha256_hex(context.as_bytes()) != enrollment.custody_context_sha256 {
        return Err("managed identity enrollment context does not match".to_string());
    }
    Ok(context)
}

fn validate_recovery_challenge(
    recovery: &RecoveryChallenge,
    device: &DeviceTransport,
    binding: &IdentityBinding,
) -> Result<(), String> {
    validate_uuid(&recovery.recovery_id, "recovery")?;
    validate_hex_64(&recovery.public_key, "public key")?;
    validate_hex_64(&recovery.device_public_key_sha256, "device key digest")?;
    validate_expiry(&recovery.expires_at)?;
    if recovery.key_epoch == 0
        || recovery.access_revision == 0
        || binding.public_key.as_deref() != Some(recovery.public_key.as_str())
        || recovery.device_public_key_sha256 != device_public_key_sha256(device)?
    {
        return Err("managed identity recovery changed the server-selected identity".to_string());
    }
    Ok(())
}

fn validate_recovery_payload(
    payload: &RecoveryPayload,
    recovery: &RecoveryChallenge,
    binding: &IdentityBinding,
    challenge: &ChallengeResponse,
) -> Result<String, String> {
    if payload.schema_version != RECOVERY_SCHEMA
        || payload.next_action != "verify_key_challenge"
        || payload.public_key != recovery.public_key
        || payload.public_key != challenge.challenge.public_key
        || payload.identity_id != challenge.challenge.identity_id
        || payload.key_epoch != recovery.key_epoch
        || payload.access_revision != recovery.access_revision
        || challenge.challenge.membership_id != binding.membership_id
    {
        return Err("managed identity recovery changed the server-selected scope".to_string());
    }
    validate_uuid(&payload.identity_id, "identity")?;
    validate_hex_64(&payload.public_key, "public key")?;
    validate_hex_64(&payload.payload_sha256, "payload digest")?;
    custody_context(
        &payload.identity_id,
        &challenge.challenge.membership_id,
        &challenge.challenge.community_id,
    )
}

fn validate_recovered_entitlement(
    entitlement: &EvaosTeamsEntitlement,
    recovery: &RecoveryChallenge,
) -> Result<(), String> {
    if entitlement.access_revision != recovery.access_revision
        || entitlement.public_key.as_deref() != Some(recovery.public_key.as_str())
    {
        return Err(
            "managed identity recovery completed against a stale access revision".to_string(),
        );
    }
    Ok(())
}

pub(super) async fn ensure_enrollment(
    client: &reqwest::Client,
    token: &str,
    binding: &IdentityBinding,
    keys: &Keys,
) -> Result<CustodyEnrollment, String> {
    let device = DeviceTransport::generate()?;
    let response: EnrollmentResponse = match post_json(
        client,
        "evaos-teams-access",
        Some(token),
        serde_json::json!({
            "action": "issue_identity_custody_enrollment",
            "device_transport_public_key": device.public_key,
        }),
    )
    .await
    {
        Ok(response) => response,
        Err(ApiFailure { status, code })
            if status == reqwest::StatusCode::FORBIDDEN
                && code == "identity_custody_enrollment_unavailable" =>
        {
            return Ok(CustodyEnrollment::AlreadyPresent);
        }
        Err(error) => {
            return Err(format!(
                "Managed identity recovery setup was not available: {error}"
            ));
        }
    };
    if response.status != "identity_custody_enrollment_issued" {
        return Err("Managed identity recovery setup returned an invalid response".to_string());
    }
    let enrollment = response.enrollment;
    let context = validate_enrollment(&enrollment, binding, keys)?;
    let info = format!("{ENROLLMENT_DEK_INFO}:{}", enrollment.challenge_id);
    let data_key = device.open(&enrollment.sealed_data_key, &info)?;
    let (payload_ciphertext, payload_nonce, payload_sha256) =
        encrypt_secret_key(keys, &data_key, &context)?;
    let signed_event = signed_enrollment_event(&enrollment, &payload_sha256, keys)?;
    let completed: EnrollmentCompletionResponse = post_json(
        client,
        "evaos-teams-access",
        Some(token),
        serde_json::json!({
            "action": "complete_identity_custody_enrollment",
            "payload_ciphertext": payload_ciphertext,
            "payload_nonce": payload_nonce,
            "signed_event": signed_event,
        }),
    )
    .await
    .map_err(|error| format!("Managed identity recovery setup was rejected: {error}"))?;
    if completed.status != "identity_custody_active" {
        return Err("Managed identity recovery setup did not become active".to_string());
    }
    Ok(CustodyEnrollment::Completed)
}

pub(super) async fn recover_identity(
    app: &tauri::AppHandle,
    app_state: &AppState,
    client: &reqwest::Client,
    token: &str,
    binding: &IdentityBinding,
) -> Result<(Keys, EvaosTeamsEntitlement), String> {
    let canonical_public_key = binding.public_key.as_deref().ok_or_else(|| {
        "Managed identity recovery is not available for a new membership".to_string()
    })?;
    validate_hex_64(canonical_public_key, "public key")?;
    let device = DeviceTransport::generate()?;
    let issued: RecoveryChallengeResponse = post_json(
        client,
        "evaos-teams-access",
        Some(token),
        serde_json::json!({
            "action": "issue_identity_recovery_challenge",
            "device_transport_public_key": device.public_key,
        }),
    )
    .await
    .map_err(|error| format!("Managed identity recovery was not available: {error}"))?;
    if issued.status != "identity_recovery_challenge_issued" {
        return Err("Managed identity recovery returned an invalid challenge".to_string());
    }
    let recovery = issued.recovery;
    validate_recovery_challenge(&recovery, &device, binding)?;
    let challenge_info = format!("{RECOVERY_CHALLENGE_INFO}:{}", recovery.recovery_id);
    let challenge_nonce = device.open(&recovery.sealed_challenge, &challenge_info)?;
    let challenge_nonce = std::str::from_utf8(&challenge_nonce)
        .map_err(|_| "Managed identity recovery challenge is invalid".to_string())?;
    if challenge_nonce.len() != 43
        || !challenge_nonce
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err("Managed identity recovery challenge is invalid".to_string());
    }
    let completed: RecoveryCompletionResponse = post_json(
        client,
        "evaos-teams-access",
        Some(token),
        serde_json::json!({
            "action": "complete_identity_recovery",
            "recovery_id": recovery.recovery_id,
            "challenge_nonce": challenge_nonce,
        }),
    )
    .await
    .map_err(|error| format!("Managed identity recovery was rejected: {error}"))?;
    if completed.status != "identity_recovery_authorized" {
        return Err("Managed identity recovery did not authorize this device".to_string());
    }
    let payload = completed.recovery;
    let key_challenge =
        issue_key_challenge(client, token, canonical_public_key, &binding.membership_id).await?;
    let context = validate_recovery_payload(&payload, &recovery, binding, &key_challenge)?;
    let data_key_info = format!("{RECOVERY_DEK_INFO}:{}", recovery.recovery_id);
    let data_key = device.open(&payload.sealed_data_key, &data_key_info)?;
    let keys = decrypt_secret_key(&payload, &data_key, &context)?;
    if keys.public_key().to_hex() != canonical_public_key {
        return Err("Recovered Hive identity does not match this membership".to_string());
    }

    let entitlement =
        verify_key_challenge(client, token, &key_challenge, &keys, &binding.membership_id).await?;
    validate_recovered_entitlement(&entitlement, &recovery)?;

    super::authorization::prepare_managed_identity_recovery(app, app_state)?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("app data dir: {error}"))?;
    std::fs::create_dir_all(&data_dir).map_err(|error| format!("create app data dir: {error}"))?;
    let key_path = data_dir.join("identity.key");
    let store = crate::secret_store::SecretStore::shared(crate::app_state::keyring_service());
    crate::app_state::persist_managed_recovered_identity(
        store, app_state, &keys, &key_path, &data_dir,
    )?;
    Ok((keys, entitlement))
}

#[cfg(test)]
mod tests {
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
        assert!(
            validate_recovery_payload(&payload, &recovery, &wrong_membership, &challenge).is_err()
        );

        let mut wrong_identity = fixture_payload(&keys);
        wrong_identity.identity_id = "10000000-0000-4000-8000-000000000098".to_string();
        assert!(
            validate_recovery_payload(&wrong_identity, &recovery, &binding, &challenge).is_err()
        );

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
        assert_eq!(
            validate_enrollment(&enrollment, &binding, &keys).unwrap(),
            context
        );

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
        let (encapped_key, ciphertext) =
            hpke::single_shot_seal::<DeviceAead, DeviceKdf, DeviceKem>(
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
}
