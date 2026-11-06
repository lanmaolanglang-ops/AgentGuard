use crate::{Epochs, Failure, Result, SecretKind, UtcMillis};
use agentguard_protocol::crypto::{self, Domain};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// One explicit generation with six independent synthetic keys. No generic signing API.
pub struct SyntheticKeys {
    pub(crate) id: String,
    pub(crate) generation: u64,
    keys: [[u8; 32]; 6],
}

impl SyntheticKeys {
    /// Key order: secret, semantic, intent, operation, permit, audit.
    pub fn new(id: String, generation: u64, keys: [[u8; 32]; 6]) -> Result<Self> {
        if id.is_empty() || id.chars().count() > 64 || generation == 0 {
            return Err(Failure::Key);
        }
        for (index, key) in keys.iter().enumerate() {
            if *key == [0; 32] || keys[..index].contains(key) {
                return Err(Failure::Key);
            }
        }
        Ok(Self {
            id,
            generation,
            keys,
        })
    }

    pub(crate) fn fingerprints(&self) -> [[u8; 32]; 6] {
        self.keys.map(|key| Sha256::digest(key).into())
    }

    fn key(&self, domain: Domain) -> &[u8; 32] {
        &self.keys[match domain {
            Domain::Secret => 0,
            Domain::Semantic => 1,
            Domain::Intent => 2,
            Domain::Operation => 3,
            Domain::Permit => 4,
            Domain::Audit => 5,
        }]
    }

    pub(crate) fn sign(&self, domain: Domain, payload: &Value) -> Result<String> {
        crypto::mac(self.key(domain), domain, payload)
            .map(|mac| crypto::hex(&mac))
            .map_err(|_| Failure::InvalidWire)
    }

    pub(crate) fn verify(&self, domain: Domain, payload: &Value, mac: &str) -> Result<()> {
        crypto::verify(self.key(domain), domain, payload, mac).map_err(|_| Failure::Mac)
    }

    pub(crate) fn semantic_payload(
        &self,
        operation: &Value,
        kinds: &[SecretKind],
    ) -> Result<Value> {
        let key_id = &operation["secret_parameters"]["key_id"];
        let entries = operation["secret_parameters"]["entries"]
            .as_array()
            .ok_or(Failure::InvalidWire)?;
        if entries.len() != kinds.len() || key_id.as_str() != Some(&self.id) {
            return Err(Failure::Key);
        }
        let mut shadow = operation.clone();
        let mut previous = None::<&str>;
        for (entry, kind) in entries.iter().zip(kinds) {
            let pointer = text(entry, "json_pointer")?;
            validate_pointer(pointer)?;
            if previous.is_some_and(|prior| prior >= pointer || is_ancestor(prior, pointer)) {
                return Err(Failure::InvalidBinding);
            }
            previous = Some(pointer);
            let value = operation
                .pointer(pointer)
                .and_then(Value::as_str)
                .ok_or(Failure::InvalidBinding)?;
            let payload = secret_payload(pointer, *kind, value)?;
            let mac = text(entry, "secret_mac")?;
            self.verify(Domain::Secret, &payload, mac)?;
            *shadow.pointer_mut(pointer).ok_or(Failure::InvalidBinding)? =
                json!({"key_id": key_id, "secret_mac": mac});
        }
        shadow
            .as_object_mut()
            .ok_or(Failure::InvalidWire)?
            .remove("invocation_id");
        Ok(shadow)
    }
}

fn is_ancestor(prior: &str, pointer: &str) -> bool {
    pointer
        .strip_prefix(prior)
        .is_some_and(|tail| tail.starts_with('/'))
}

fn validate_pointer(pointer: &str) -> Result<()> {
    if pointer != "/command" && !pointer.starts_with("/arguments/") && pointer != "/arguments" {
        return Err(Failure::InvalidBinding);
    }
    let mut bytes = pointer.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'~' && !matches!(bytes.next(), Some(b'0' | b'1')) {
            return Err(Failure::InvalidBinding);
        }
    }
    Ok(())
}

pub(crate) fn secret_payload(pointer: &str, kind: SecretKind, value: &str) -> Result<Value> {
    let kind = match kind {
        SecretKind::Utf8 => "utf8",
        SecretKind::Base64Bytes if canonical_base64(value) => "base64_bytes",
        SecretKind::Base64Bytes => return Err(Failure::InvalidBinding),
    };
    Ok(json!({"parameter_pointer": pointer, "value_kind": kind, "value": value}))
}

fn canonical_base64(value: &str) -> bool {
    let body = value.trim_end_matches('=');
    let padding = value.len() - body.len();
    if !value.len().is_multiple_of(4) || padding > 2 {
        return false;
    }
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    if !body.bytes().all(|byte| ALPHABET.contains(&byte)) {
        return false;
    }
    if padding == 0 {
        return true;
    }
    let index = ALPHABET
        .iter()
        .position(|&byte| Some(&byte) == body.as_bytes().last());
    index.is_some_and(|index| index.is_multiple_of(if padding == 1 { 4 } else { 16 }))
}

pub(crate) fn intent_payload(semantic_digest: &str, epochs: Epochs, expiry: UtcMillis) -> Value {
    json!({
        "semantic_digest": semantic_digest, "policy_epoch": epochs.policy.0,
        "grant_epoch": epochs.grant.0, "core_boot_epoch": epochs.core.0.to_string(),
        "expires_at": expiry.rfc3339(), "remaining_claims": 1,
    })
}

pub(crate) fn operation_payload(permit: &Value) -> Value {
    const FIELDS: [&str; 14] = [
        "semantic_digest",
        "invocation_id",
        "grant_id",
        "reservation_id",
        "broker_instance",
        "prepared_id",
        "nonce",
        "key_id",
        "core_boot_epoch",
        "policy_epoch",
        "grant_epoch",
        "expires_at",
        "ttl_ms",
        "max_uses",
    ];
    Value::Object(
        FIELDS
            .into_iter()
            .map(|key| (key.into(), permit[key].clone()))
            .collect(),
    )
}

pub(crate) fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key].as_str().ok_or(Failure::InvalidWire)
}

pub(crate) fn digest(value: &Value) -> Result<[u8; 32]> {
    let encoded = crypto::typed_encode(value).map_err(|_| Failure::InvalidWire)?;
    Ok(Sha256::digest(encoded).into())
}

#[derive(Clone, Copy)]
pub struct EventDigest([u8; 32]);

/// Input must already be a redacted synthetic event. This helper does not perform redaction.
pub fn redacted_event_digest(redacted_json: &[u8]) -> Result<EventDigest> {
    let event =
        agentguard_protocol::parse::parse_json(redacted_json).map_err(|_| Failure::InvalidWire)?;
    if !event.is_object() {
        return Err(Failure::InvalidWire);
    }
    Ok(EventDigest(digest(&event)?))
}

pub struct AuditLink<'a> {
    pub segment_id: Uuid,
    pub sequence: u64,
    pub previous_mac: Option<&'a str>,
    pub event_digest: EventDigest,
    pub key_id: &'a str,
    pub key_generation: u64,
}

impl AuditLink<'_> {
    pub(crate) fn payload(&self, keys: &SyntheticKeys) -> Result<Value> {
        crate::model::require_ids(&[self.segment_id])?;
        if self.key_id != keys.id || self.key_generation != keys.generation {
            return Err(Failure::Key);
        }
        if let Some(previous) = self.previous_mac {
            crypto::decode_mac(previous).map_err(|_| Failure::Mac)?;
        }
        Ok(json!({
            "segment_id": self.segment_id.to_string(), "sequence": self.sequence,
            "previous_mac": self.previous_mac, "event_digest": crypto::hex(&self.event_digest.0),
            "key_id": self.key_id,
        }))
    }
}
