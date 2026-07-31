use std::collections::HashMap;

use nostr::{Keys, ToBech32};

use super::{
    LEGACY_ACTIVE_MEMBERSHIP_KEY, LEGACY_IDENTITY_KEY, LEGACY_IDENTITY_KEY_PREFIX,
    LOGOUT_PENDING_KEY, SESSION_KEY,
};

const IDENTITY_ROTATION_KEY_PREFIX: &str = "pending_identity_rotation:";

fn is_legacy_identity_key(key: &str) -> bool {
    key == LEGACY_IDENTITY_KEY
        || key
            .strip_prefix(LEGACY_IDENTITY_KEY_PREFIX)
            .is_some_and(|membership_id| uuid::Uuid::parse_str(membership_id).is_ok())
}

fn pending_identity_rotation_membership(key: &str) -> Option<&str> {
    key.strip_prefix(IDENTITY_ROTATION_KEY_PREFIX)
        .filter(|membership_id| uuid::Uuid::parse_str(membership_id).is_ok())
}

pub(super) fn pending_identity_rotation_key(membership_id: &str) -> Result<String, String> {
    uuid::Uuid::parse_str(membership_id)
        .map_err(|_| "managed membership identity is invalid".to_string())?;
    Ok(format!("{IDENTITY_ROTATION_KEY_PREFIX}{membership_id}"))
}

pub(super) fn validated_runtime_entries(
    stored: Option<HashMap<String, String>>,
) -> Result<HashMap<String, String>, String> {
    let stored = stored.unwrap_or_default();
    let has_unsupported = stored.keys().any(|key| {
        key != SESSION_KEY
            && key != LOGOUT_PENDING_KEY
            && key != LEGACY_ACTIVE_MEMBERSHIP_KEY
            && !is_legacy_identity_key(key.as_str())
            && pending_identity_rotation_membership(key.as_str()).is_none()
    });
    if has_unsupported {
        return Err("managed Keychain contains unsupported credential material".to_string());
    }
    Ok(stored)
}

pub(super) fn preserve_legacy_identity_entries(
    stored: &HashMap<String, String>,
) -> Result<HashMap<String, String>, String> {
    let mut preserved = validated_runtime_entries(Some(stored.clone()))?;
    preserved.retain(|key, _| {
        key == LEGACY_ACTIVE_MEMBERSHIP_KEY
            || is_legacy_identity_key(key.as_str())
            || pending_identity_rotation_membership(key.as_str()).is_some()
    });
    Ok(preserved)
}

pub(super) fn staged_identity_rotation_entries(
    stored: &HashMap<String, String>,
    membership_id: &str,
) -> Result<(HashMap<String, String>, Keys), String> {
    let mut replacement = validated_runtime_entries(Some(stored.clone()))?;
    let staging_key = pending_identity_rotation_key(membership_id)?;
    let keys = replacement
        .get(&staging_key)
        .map(|value| {
            Keys::parse(value.trim())
                .map_err(|_| "staged Hive identity in Keychain is invalid".to_string())
        })
        .transpose()?
        .unwrap_or_else(Keys::generate);
    let encoded = keys
        .secret_key()
        .to_bech32()
        .map_err(|error| format!("could not encode replacement identity: {error}"))?;
    replacement.insert(staging_key, encoded);
    Ok((replacement, keys))
}

pub(super) fn pending_session_entries(session: &str) -> HashMap<String, String> {
    HashMap::from([
        (SESSION_KEY.to_string(), session.to_string()),
        (LOGOUT_PENDING_KEY.to_string(), "1".to_string()),
    ])
}

pub(super) fn select_legacy_identity_candidate(
    stored: &HashMap<String, String>,
    membership_id: &str,
    canonical_public_key: &str,
) -> Result<Option<Keys>, String> {
    uuid::Uuid::parse_str(membership_id)
        .map_err(|_| "Electric Sheep returned an invalid membership".to_string())?;
    let stored = validated_runtime_entries(Some(stored.clone()))?;
    let scoped_key = format!("{LEGACY_IDENTITY_KEY_PREFIX}{membership_id}");
    let mut matched = None;

    for key in [scoped_key.as_str(), LEGACY_IDENTITY_KEY] {
        let Some(value) = stored.get(key) else {
            continue;
        };
        let Ok(candidate) = Keys::parse(value.trim()) else {
            continue;
        };
        if candidate.public_key().to_hex() != canonical_public_key {
            continue;
        }
        if matched
            .as_ref()
            .is_some_and(|existing: &Keys| existing.public_key() != candidate.public_key())
        {
            return Err("managed Keychain contains conflicting canonical identities".to_string());
        }
        matched = Some(candidate);
    }

    Ok(matched)
}

pub(super) fn active_session_entries(
    stored: &HashMap<String, String>,
    session: &str,
    membership_id: &str,
    adopted_public_key: &str,
) -> Result<HashMap<String, String>, String> {
    uuid::Uuid::parse_str(membership_id)
        .map_err(|_| "Electric Sheep returned an invalid membership".to_string())?;
    let mut replacement = validated_runtime_entries(Some(stored.clone()))?;
    let matching_keys: Vec<String> = replacement
        .iter()
        .filter_map(|(key, value)| {
            if !is_legacy_identity_key(key) {
                return None;
            }
            Keys::parse(value.trim())
                .ok()
                .filter(|keys| keys.public_key().to_hex() == adopted_public_key)
                .map(|_| key.clone())
        })
        .collect();
    let removed_matching_identity = !matching_keys.is_empty();
    for key in matching_keys {
        replacement.remove(&key);
    }
    if removed_matching_identity
        && replacement
            .get(LEGACY_ACTIVE_MEMBERSHIP_KEY)
            .is_some_and(|active| active == membership_id)
    {
        replacement.remove(LEGACY_ACTIVE_MEMBERSHIP_KEY);
    }
    let rotation_key = pending_identity_rotation_key(membership_id)?;
    if replacement
        .get(&rotation_key)
        .and_then(|value| Keys::parse(value.trim()).ok())
        .is_some_and(|keys| keys.public_key().to_hex() == adopted_public_key)
    {
        replacement.remove(&rotation_key);
    }
    replacement.remove(LOGOUT_PENDING_KEY);
    replacement.insert(SESSION_KEY.to_string(), session.to_string());
    Ok(replacement)
}
