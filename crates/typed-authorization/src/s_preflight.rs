//! Pure, typed S1 checks. This module owns no grants, counters or commit state.
use crate::crypto::{digest, operation_payload, text};
use crate::model::require_ids;
use crate::snapshot::{Snapshot, parse_uuid};
use crate::*;
use agentguard_protocol::{
    crypto::{self, Domain},
    parse::parse_json,
    schema::Schemas,
};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

/// Explicit synthetic Core evidence. Identifiers do not authenticate Windows peers.
pub struct ReserveInput {
    pub grant: Uuid,
    pub approval: Uuid,
    pub approval_version: u32,
    pub parent: Option<RuleRef>,
    pub epochs: Epochs,
    pub preparation: Preparation,
    pub decision: Decision,
    pub operation: Vec<u8>,
    pub secret_kinds: Vec<SecretKind>,
    pub ttl_ms: u32,
}

/// Private construction and no Clone, Debug, serde or authorization conversion.
pub struct PrecommitCandidate {
    input: ReserveInput,
    fingerprint: [u8; 32],
}

pub struct ConsumeCandidate {
    context: PrecommitCandidate,
    permit: Value,
    fingerprint: [u8; 32],
}

pub fn preflight_s_reserve(input: ReserveInput) -> Result<PrecommitCandidate> {
    let p = input.preparation;
    require_ids(&[
        input.grant,
        input.approval,
        input.preparation.id,
        input.decision.id,
        p.peer.connection,
        p.peer.broker_instance,
        p.peer.worker_instance,
    ])?;
    input.epochs.validate()?;
    input.decision.eligible()?;
    if input.approval_version == 0
        || !(1..=60_000).contains(&input.ttl_ms)
        || p.generation == 0
        || p.peer.broker_instance == p.peer.worker_instance
        || input
            .parent
            .is_some_and(|rule| rule.id.is_nil() || rule.version == 0)
    {
        return Err(Failure::InvalidBinding);
    }
    let value = parse_json(&input.operation).map_err(|_| Failure::InvalidWire)?;
    Schemas::new()
        .map_err(|_| Failure::InvalidWire)?
        .validate("operation-v2", &value)
        .map_err(|_| Failure::InvalidWire)?;
    let reported_boot = parse_uuid(
        &value["requestor_principal"]["process_instance"],
        "os_boot_id",
    )?;
    require_ids(&[reported_boot])?;
    Snapshot::validate_evidence(&value, reported_boot)?;
    let fingerprint = digest(&json!({
        "domain": "AgentGuard/S1/candidate", "grant": input.grant.to_string(),
        "epochs": [json!(input.epochs.policy.0), json!(input.epochs.grant.0), json!(input.epochs.core.0.to_string())],
        "prepared": [json!(p.id.to_string()), json!(p.generation), json!(p.deadline.utc.unix_millis()), json!(p.deadline.monotonic_ms)],
        "peer": [p.peer.connection.to_string(),p.peer.broker_instance.to_string(),p.peer.worker_instance.to_string()],
        "decision": input.decision.id.to_string(), "operation": value,
        "secret_kinds": input.secret_kinds.iter().map(|k| match k {SecretKind::Utf8 => 0, SecretKind::Base64Bytes => 1}).collect::<Vec<_>>()
    }))?;
    Ok(PrecommitCandidate { input, fingerprint })
}

pub fn preflight_s_consume(context: PrecommitCandidate, wire: &[u8]) -> Result<ConsumeCandidate> {
    let permit = parse_json(wire).map_err(|_| Failure::InvalidWire)?;
    Schemas::new()
        .map_err(|_| Failure::InvalidWire)?
        .validate("permit-v2", &permit)
        .map_err(|_| Failure::InvalidWire)?;
    let fingerprint = digest(
        &json!({"domain": "AgentGuard/S1/consume", "context": crypto::hex(&context.fingerprint), "permit": permit}),
    )?;
    Ok(ConsumeCandidate {
        context,
        permit,
        fingerprint,
    })
}

impl PrecommitCandidate {
    pub fn into_input(self) -> ReserveInput {
        self.input
    }
    pub fn input(&self) -> &ReserveInput {
        &self.input
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn request_fingerprint(&self) -> Result<[u8; 32]> {
        digest(
            &json!({"binding": crypto::hex(&self.fingerprint), "approval": self.input.approval.to_string(),
            "version":self.input.approval_version, "parent":self.input.parent.map(|r|(r.id.to_string(),r.version)), "ttl":self.input.ttl_ms}),
        )
    }
}
impl ConsumeCandidate {
    pub fn context(&self) -> &PrecommitCandidate {
        &self.context
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn reservation(&self) -> Result<Uuid> {
        parse_uuid(&self.permit, "reservation_id")
    }
}

/// Core-owned keys move into the sole Writer. No general payload signing entry.
pub struct WriterVerifier {
    keys: SyntheticKeys,
    schemas: Schemas,
}
pub struct VerifiedCandidate<'a> {
    candidate: &'a PrecommitCandidate,
    snapshot: Snapshot,
}
pub struct SignedReservation {
    pub operation_mac: String,
    pub permit_mac: String,
    pub nonce_hash: [u8; 32],
    pub deadline: Deadline,
    pub wire_digest: [u8; 32],
    permit: Permit,
}
impl SignedReservation {
    pub fn into_permit(self) -> Permit {
        self.permit
    }
}

/// Immutable transaction facts read by the Writer, with no verification boolean.
pub struct ReservationFacts<'a> {
    pub id: Uuid,
    pub nonce_hash: [u8; 32],
    pub deadline: Deadline,
    pub operation_mac: &'a str,
    pub permit_mac: &'a str,
}

impl WriterVerifier {
    pub fn new(keys: SyntheticKeys) -> Result<Self> {
        Ok(Self {
            keys,
            schemas: Schemas::new().map_err(|_| Failure::InvalidWire)?,
        })
    }
    pub fn key_id(&self) -> &str {
        &self.keys.id
    }
    pub fn generation(&self) -> u64 {
        self.keys.generation
    }
    pub fn key_fingerprints(&self) -> [[u8; 32]; 6] {
        self.keys.fingerprints()
    }

    pub fn verify_reserve<'a>(
        &self,
        candidate: &'a PrecommitCandidate,
        now: ClockSample,
    ) -> Result<VerifiedCandidate<'a>> {
        let i = &candidate.input;
        if !i.preparation.deadline.live(now) {
            return Err(Failure::Expired);
        }
        let observation = Observation {
            preparation: i.preparation,
            decision: i.decision,
            operation_json: &i.operation,
            secret_kinds: &i.secret_kinds,
        };
        let snapshot = Snapshot::read(&self.schemas, &self.keys, &observation, now.os_boot_id)?;
        Ok(VerifiedCandidate {
            candidate,
            snapshot,
        })
    }

    pub fn sign_reservation(
        &self,
        verified: VerifiedCandidate<'_>,
        id: Uuid,
        nonce: [u8; 32],
        now: ClockSample,
    ) -> Result<SignedReservation> {
        require_ids(&[id])?;
        if nonce == [0; 32] {
            return Err(Failure::InvalidBinding);
        }
        let deadline = Deadline::after(now, u64::from(verified.candidate.input.ttl_ms))?;
        if !deadline.within(verified.candidate.input.preparation.deadline) {
            return Err(Failure::Expired);
        }
        let mut payload = self.permit_payload(&verified, id, &crypto::hex(&nonce), deadline)?;
        let operation_mac = self
            .keys
            .sign(Domain::Operation, &operation_payload(&payload))?;
        payload["operation_digest"] = json!(operation_mac);
        let permit_mac = self.keys.sign(Domain::Permit, &payload)?;
        payload["mac"] = json!(permit_mac);
        self.schemas
            .validate("permit-v2", &payload)
            .map_err(|_| Failure::InvalidWire)?;
        Ok(SignedReservation {
            operation_mac,
            permit_mac,
            nonce_hash: Sha256::digest(nonce).into(),
            deadline,
            wire_digest: digest(&payload)?,
            permit: Permit(payload),
        })
    }

    pub fn verify_consume(
        &self,
        candidate: &ConsumeCandidate,
        facts: ReservationFacts<'_>,
        now: ClockSample,
    ) -> Result<()> {
        let verified = self.verify_reserve(&candidate.context, now)?;
        if !facts.deadline.live(now) {
            return Err(Failure::Expired);
        }
        let mut payload = candidate.permit.clone();
        let mac = payload
            .as_object_mut()
            .ok_or(Failure::InvalidWire)?
            .remove("mac")
            .ok_or(Failure::InvalidWire)?;
        self.keys.verify(
            Domain::Permit,
            &payload,
            mac.as_str().ok_or(Failure::InvalidWire)?,
        )?;
        let operation = operation_payload(&payload);
        self.keys.verify(
            Domain::Operation,
            &operation,
            text(&payload, "operation_digest")?,
        )?;
        self.keys
            .verify(Domain::Operation, &operation, facts.operation_mac)?;
        self.keys
            .verify(Domain::Permit, &payload, facts.permit_mac)?;
        let nonce =
            crypto::decode_mac(text(&payload, "nonce")?).map_err(|_| Failure::InvalidWire)?;
        if <[u8; 32]>::from(Sha256::digest(nonce)) != facts.nonce_hash {
            return Err(Failure::InvalidBinding);
        }
        let mut expected = self.permit_payload(
            &verified,
            facts.id,
            text(&payload, "nonce")?,
            facts.deadline,
        )?;
        expected["operation_digest"] = json!(facts.operation_mac);
        if expected != payload {
            return Err(Failure::InvalidBinding);
        }
        Ok(())
    }

    fn permit_payload(
        &self,
        verified: &VerifiedCandidate<'_>,
        id: Uuid,
        nonce: &str,
        deadline: Deadline,
    ) -> Result<Value> {
        let i = &verified.candidate.input;
        let mut value = verified.snapshot.permit_identity()?;
        let fields = json!({
            "schema_version":"2.0.0", "grant_id": i.grant.to_string(), "reservation_id": id.to_string(),
            "nonce": nonce, "key_id": self.keys.id, "core_boot_epoch": i.epochs.core.0.to_string(),
            "broker_instance": i.preparation.peer.broker_instance.to_string(), "prepared_id": i.preparation.id.to_string(),
            "semantic_digest": verified.snapshot.semantic, "operation_digest": "", "policy_epoch": i.epochs.policy.0,
            "grant_epoch": i.epochs.grant.0, "expires_at": deadline.utc.rfc3339(), "ttl_ms": i.ttl_ms, "max_uses":1
        });
        value
            .as_object_mut()
            .ok_or(Failure::InvalidWire)?
            .extend(fields.as_object().ok_or(Failure::InvalidWire)?.clone());
        Ok(value)
    }

    pub fn audit_mac(&self, link: &AuditLink<'_>) -> Result<String> {
        self.keys.sign(Domain::Audit, &link.payload(&self.keys)?)
    }
    pub fn verify_audit(&self, link: &AuditLink<'_>, mac: &str) -> Result<()> {
        self.keys
            .verify(Domain::Audit, &link.payload(&self.keys)?, mac)
    }
}

impl VerifiedCandidate<'_> {
    pub fn invocation(&self) -> Uuid {
        self.snapshot.invocation
    }
    pub fn semantic(&self) -> &str {
        &self.snapshot.semantic
    }
    pub fn scope_digest(&self) -> Result<[u8; 32]> {
        digest(&self.snapshot.scope())
    }
}
