use std::collections::HashMap;

use nostr::{Keys, ToBech32};

use super::{IDENTITY_KEY, LOGOUT_PENDING_KEY, PREVIOUS_SESSION_KEY, SESSION_KEY};

pub(super) fn managed_credential_entries(
    keys: &Keys,
    session: &str,
    logout_pending: bool,
    previous_session: Option<&str>,
) -> Result<HashMap<String, String>, String> {
    let identity = keys
        .secret_key()
        .to_bech32()
        .map_err(|error| format!("could not encode managed identity: {error}"))?;
    let mut entries = HashMap::from([
        (IDENTITY_KEY.to_string(), identity),
        (SESSION_KEY.to_string(), session.to_string()),
    ]);
    if logout_pending {
        entries.insert(LOGOUT_PENDING_KEY.to_string(), "1".to_string());
    }
    if let Some(previous_session) = previous_session.filter(|value| !value.trim().is_empty()) {
        entries.insert(
            PREVIOUS_SESSION_KEY.to_string(),
            previous_session.to_string(),
        );
    }
    Ok(entries)
}
