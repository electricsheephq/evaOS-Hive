//! Optional signed company-agent responder policy.
//!
//! The policy can only narrow native relay membership. It never supplies a
//! tenant, agent selector, relay, runtime, model, prompt, credential, or key.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use uuid::Uuid;

use crate::relay::RestClient;

pub(crate) const POLICY_POLL_INTERVAL: Duration = Duration::from_secs(5);
pub(crate) const POLICY_STALE_AFTER: Duration = Duration::from_secs(15);
const POLICY_SCHEMA: &str = "hive.company_agent_responder_policy.v1";
const MAX_SELECTORS: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
struct PolicySnapshot {
    revision: u64,
    room_ids: HashSet<Uuid>,
    author_pubkeys: HashSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PolicyAck {
    pub revision: u64,
    pub result: &'static str,
    pub error_code: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PolicyUpdate {
    pub changed: bool,
    pub ack: Option<PolicyAck>,
}

#[derive(Debug)]
pub(crate) struct ResponderPolicy {
    endpoint: Option<String>,
    snapshot: Option<PolicySnapshot>,
    accepted_watermark: Option<PolicySnapshot>,
    refreshed_at: Option<Instant>,
}

impl ResponderPolicy {
    pub fn new(endpoint: Option<String>) -> Self {
        Self {
            endpoint,
            snapshot: None,
            accepted_watermark: None,
            refreshed_at: None,
        }
    }

    pub fn enabled(&self) -> bool {
        self.endpoint.is_some()
    }

    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }

    pub fn selected_rooms(&self) -> HashSet<Uuid> {
        self.snapshot
            .as_ref()
            .map(|snapshot| snapshot.room_ids.clone())
            .unwrap_or_default()
    }

    pub fn current_revision(&self) -> Option<u64> {
        self.snapshot.as_ref().map(|snapshot| snapshot.revision)
    }

    pub fn permits(&self, room_id: Uuid, author: &str, is_dm: bool, now: Instant) -> bool {
        if !self.enabled() || is_dm || self.is_stale(now) {
            return false;
        }
        self.snapshot.as_ref().is_some_and(|snapshot| {
            snapshot.room_ids.contains(&room_id) && snapshot.author_pubkeys.contains(author)
        })
    }

    pub fn selects_room(&self, room_id: Uuid, now: Instant) -> bool {
        !self.is_stale(now)
            && self
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.room_ids.contains(&room_id))
    }

    pub fn expiry_deadline(&self) -> Option<Instant> {
        self.refreshed_at.map(|at| at + POLICY_STALE_AFTER)
    }

    pub fn expire_if_stale(&mut self, now: Instant) -> bool {
        if !self.is_stale(now) {
            return false;
        }
        self.refreshed_at = None;
        self.snapshot.take().is_some()
    }

    pub fn apply_pull(&mut self, value: Value, now: Instant) -> PolicyUpdate {
        match parse_snapshot(&value) {
            Ok(next) => {
                let changed = self.snapshot.as_ref() != Some(&next);
                let ack = if next.revision > 0 && changed {
                    Some(PolicyAck {
                        revision: next.revision,
                        result: "applied",
                        error_code: None,
                    })
                } else {
                    None
                };
                if let Some(current) = &self.accepted_watermark {
                    if next.revision < current.revision
                        || (next.revision == current.revision && next != *current)
                    {
                        let revision = next.revision;
                        let changed = self.snapshot.take().is_some();
                        self.refreshed_at = None;
                        return PolicyUpdate {
                            changed,
                            ack: (revision > 0).then_some(PolicyAck {
                                revision,
                                result: "error",
                                error_code: Some("revision_content_mismatch"),
                            }),
                        };
                    }
                }
                self.accepted_watermark = Some(next.clone());
                self.snapshot = Some(next);
                self.refreshed_at = Some(now);
                PolicyUpdate { changed, ack }
            }
            Err(error_code) => {
                let revision = value.get("desired_revision").and_then(Value::as_u64);
                let changed = self.snapshot.take().is_some();
                self.refreshed_at = None;
                PolicyUpdate {
                    changed,
                    ack: revision
                        .filter(|revision| *revision > 0)
                        .map(|revision| PolicyAck {
                            revision,
                            result: "error",
                            error_code: Some(error_code),
                        }),
                }
            }
        }
    }

    fn is_stale(&self, now: Instant) -> bool {
        self.refreshed_at
            .is_none_or(|refreshed| now.saturating_duration_since(refreshed) >= POLICY_STALE_AFTER)
    }
}

fn parse_snapshot(value: &Value) -> Result<PolicySnapshot, &'static str> {
    if value.get("schema_version").and_then(Value::as_str) != Some(POLICY_SCHEMA) {
        return Err("invalid_policy_payload");
    }
    let revision = value
        .get("desired_revision")
        .and_then(Value::as_u64)
        .ok_or("invalid_policy_payload")?;
    let rooms = value
        .get("allowed_room_ids")
        .and_then(Value::as_array)
        .ok_or("invalid_policy_payload")?;
    let authors = value
        .get("allowed_author_public_keys")
        .and_then(Value::as_array)
        .ok_or("invalid_policy_payload")?;
    if rooms.len() > MAX_SELECTORS || authors.len() > MAX_SELECTORS {
        return Err("invalid_policy_payload");
    }

    let mut room_ids = HashSet::with_capacity(rooms.len());
    for room in rooms {
        let room = room
            .as_str()
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or("invalid_policy_payload")?;
        if !room_ids.insert(room) {
            return Err("invalid_policy_payload");
        }
    }

    let mut author_pubkeys = HashSet::with_capacity(authors.len());
    for author in authors {
        let author = author.as_str().ok_or("invalid_policy_payload")?;
        if author.len() != 64
            || !author
                .chars()
                .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
            || !author_pubkeys.insert(author.to_string())
        {
            return Err("invalid_policy_payload");
        }
    }

    Ok(PolicySnapshot {
        revision,
        room_ids,
        author_pubkeys,
    })
}

pub(crate) async fn pull_policy(rest: &RestClient, endpoint: &str) -> Result<Value, String> {
    rest.post_external_nip98_json(endpoint, &json!({ "action": "pull_policy" }))
        .await
        .map_err(|error| error.to_string())
}

pub(crate) async fn acknowledge_policy(
    rest: &RestClient,
    endpoint: &str,
    ack: &PolicyAck,
) -> Result<(), String> {
    let mut body = json!({
        "action": "ack_policy",
        "desired_revision": ack.revision,
        "result": ack.result,
    });
    if let Some(error_code) = ack.error_code {
        body["error_code"] = Value::String(error_code.to_string());
    }
    rest.post_external_nip98_json(endpoint, &body)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(revision: u64, rooms: Vec<String>, authors: Vec<String>) -> Value {
        json!({
            "schema_version": POLICY_SCHEMA,
            "desired_revision": revision,
            "allowed_room_ids": rooms,
            "allowed_author_public_keys": authors,
        })
    }

    #[test]
    fn pending_and_expired_policy_fail_closed() {
        let now = Instant::now();
        let room = Uuid::new_v4();
        let author = "a".repeat(64);
        let mut policy = ResponderPolicy::new(Some("https://example.test/policy".to_string()));
        assert!(!policy.permits(room, &author, false, now));
        let update = policy.apply_pull(value(1, vec![room.to_string()], vec![author.clone()]), now);
        assert!(update.changed);
        assert!(policy.permits(room, &author, false, now));
        assert!(policy.expire_if_stale(now + POLICY_STALE_AFTER));
        assert!(!policy.permits(room, &author, false, now + POLICY_STALE_AFTER));
    }

    #[test]
    fn exact_room_author_and_non_dm_are_all_required() {
        let now = Instant::now();
        let room = Uuid::new_v4();
        let other_room = Uuid::new_v4();
        let author = "a".repeat(64);
        let mut policy = ResponderPolicy::new(Some("https://example.test/policy".to_string()));
        policy.apply_pull(value(1, vec![room.to_string()], vec![author.clone()]), now);
        assert!(policy.permits(room, &author, false, now));
        assert!(!policy.permits(other_room, &author, false, now));
        assert!(!policy.permits(room, &"b".repeat(64), false, now));
        assert!(!policy.permits(room, &author, true, now));
    }

    #[test]
    fn lower_or_mutated_same_revision_fails_closed() {
        let now = Instant::now();
        let room = Uuid::new_v4();
        let author = "a".repeat(64);
        let mut policy = ResponderPolicy::new(Some("https://example.test/policy".to_string()));
        policy.apply_pull(value(2, vec![room.to_string()], vec![author.clone()]), now);
        let update = policy.apply_pull(value(1, vec![], vec![]), now);
        assert!(update.changed);
        assert_eq!(update.ack.unwrap().result, "error");
        assert!(!policy.permits(room, &author, false, now));

        let repeated_stale = policy.apply_pull(value(1, vec![], vec![]), now);
        assert_eq!(
            repeated_stale.ack.unwrap().error_code,
            Some("revision_content_mismatch")
        );
        assert!(!policy.permits(room, &author, false, now));

        policy.apply_pull(value(2, vec![room.to_string()], vec![author.clone()]), now);
        let update = policy.apply_pull(value(2, vec![], vec![]), now);
        assert_eq!(
            update.ack.unwrap().error_code,
            Some("revision_content_mismatch")
        );
        assert!(!policy.permits(room, &author, false, now));

        let repeated_mutation = policy.apply_pull(value(2, vec![], vec![]), now);
        assert_eq!(
            repeated_mutation.ack.unwrap().error_code,
            Some("revision_content_mismatch")
        );
        assert!(!policy.permits(room, &author, false, now));
    }

    #[test]
    fn revision_zero_is_valid_deny_all_without_ack() {
        let now = Instant::now();
        let mut policy = ResponderPolicy::new(Some("https://example.test/policy".to_string()));
        let update = policy.apply_pull(value(0, vec![], vec![]), now);
        assert!(update.changed);
        assert!(update.ack.is_none());
        assert!(policy.selected_rooms().is_empty());
    }

    #[test]
    fn invalid_payload_is_rejected_and_ack_has_no_scope_selectors() {
        let now = Instant::now();
        let mut policy = ResponderPolicy::new(Some("https://example.test/policy".to_string()));
        let update = policy.apply_pull(
            json!({
                "schema_version": POLICY_SCHEMA,
                "desired_revision": 3,
                "allowed_room_ids": ["not-a-uuid"],
                "allowed_author_public_keys": [],
            }),
            now,
        );
        let ack = update.ack.unwrap();
        assert_eq!(ack.revision, 3);
        assert_eq!(ack.error_code, Some("invalid_policy_payload"));
        let body = json!({
            "action": "ack_policy",
            "desired_revision": ack.revision,
            "result": ack.result,
            "error_code": ack.error_code,
        });
        assert!(body.get("agent_instance_id").is_none());
        assert!(body.get("community_id").is_none());
        assert!(body.get("public_key").is_none());
    }
}
