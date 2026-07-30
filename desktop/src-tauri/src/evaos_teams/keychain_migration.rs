use std::collections::HashMap;

use super::{
    LEGACY_ACTIVE_MEMBERSHIP_KEY, LEGACY_IDENTITY_KEY, LEGACY_IDENTITY_KEY_PREFIX,
    LOGOUT_PENDING_KEY, SESSION_KEY,
};

pub(super) fn normalized_runtime_entries(
    stored: Option<HashMap<String, String>>,
) -> Result<(HashMap<String, String>, bool), String> {
    let mut stored = stored.unwrap_or_default();
    let is_legacy_identity_key = |key: &str| {
        key == LEGACY_IDENTITY_KEY
            || key
                .strip_prefix(LEGACY_IDENTITY_KEY_PREFIX)
                .is_some_and(|membership_id| uuid::Uuid::parse_str(membership_id).is_ok())
    };
    let has_unsupported = stored.keys().any(|key| {
        key != SESSION_KEY
            && key != LOGOUT_PENDING_KEY
            && key != LEGACY_ACTIVE_MEMBERSHIP_KEY
            && !is_legacy_identity_key(key)
    });
    if has_unsupported {
        return Err("managed Keychain contains unsupported credential material".to_string());
    }
    let original_len = stored.len();
    stored.retain(|key, _| key == SESSION_KEY || key == LOGOUT_PENDING_KEY);
    let removed_legacy_identity = stored.len() != original_len;
    Ok((stored, removed_legacy_identity))
}
