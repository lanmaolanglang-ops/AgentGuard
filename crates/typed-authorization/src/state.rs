use crate::crypto::{intent_payload, operation_payload, text};
use crate::model::{require_ids, require_role};
use crate::snapshot::{Snapshot, parse_uuid};
use crate::*;
use agentguard_protocol::{
    crypto::{self, Domain},
    parse::parse_json,
    schema::Schemas,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

struct ParentRule {
    spec: RuleSpec,
    scope: Value,
    epochs: Epochs,
    consumed_operations: u32,
    revoked: bool,
}

impl ParentRule {
    fn active(&self, reference: RuleRef, epochs: Epochs, now: ClockSample) -> Result<()> {
        if self.spec.reference != reference
            || self.epochs != epochs
            || self.revoked
            || now.utc < self.spec.valid_from
            || now.utc >= self.spec.expires_at
        {
            return Err(Failure::RuleUnavailable);
        }
        Ok(())
    }
}

struct Prepared {
    binding: Preparation,
    decision: Decision,
    snapshot: Option<Snapshot>,
    grant_id: Option<Uuid>,
    epochs: Epochs,
}

struct Grant {
    status: GrantStatus,
    approval: ManualApproval,
    prepared_id: Uuid,
    decision_id: Uuid,
    invocation_id: Uuid,
    epochs: Epochs,
    intent_mac: String,
    reservation: Option<Reservation>,
    unconfirmed: bool,
}

impl Grant {
    fn transition(&mut self, state: GrantState, reason: Option<EndReason>) -> Result<()> {
        if !self.status.state.permits(state) {
            return Err(Failure::InvalidState);
        }
        self.status = GrantStatus { state, reason };
        Ok(())
    }
}

/// Immutable data after reservation. No raw nonce or outbound Permit is stored.
struct Reservation {
    id: Uuid,
    grant_id: Uuid,
    decision_id: Uuid,
    approval_id: Uuid,
    preparation: Preparation,
    invocation_id: Uuid,
    semantic_mac: String,
    operation_mac: String,
    manifest: [u8; 32],
    nonce_hash: [u8; 32],
    key_id: String,
    key_generation: u64,
    os_boot_id: Uuid,
    epochs: Epochs,
    deadline: Deadline,
    ttl_ms: u32,
}

/// An in-memory Core model. Exclusivity is Rust's mutable borrow, not an OS trust boundary.
pub struct SyntheticCore {
    schemas: Schemas,
    keys: SyntheticKeys,
    epochs: Epochs,
    high_water: ClockSample,
    boot_fenced: bool,
    core_epochs: HashSet<CoreEpoch>,
    os_boots: HashSet<Uuid>,
    key_ids: HashSet<String>,
    key_history: HashSet<[u8; 32]>,
    rules: HashMap<Uuid, ParentRule>,
    prepared: HashMap<Uuid, Prepared>,
    grants: HashMap<Uuid, Grant>,
    decisions: HashSet<Uuid>,
    invocations: HashSet<Uuid>,
    approvals: HashSet<Uuid>,
    reservation_ids: HashSet<Uuid>,
    nonces: HashSet<[u8; 32]>,
}

impl SyntheticCore {
    pub fn new(epochs: Epochs, keys: SyntheticKeys, clock: &impl Clock) -> Result<Self> {
        epochs.validate()?;
        let now = clock.sample()?;
        require_ids(&[now.os_boot_id])?;
        Ok(Self {
            schemas: Schemas::new().map_err(|_| Failure::InvalidWire)?,
            epochs,
            high_water: now,
            boot_fenced: false,
            core_epochs: [epochs.core].into(),
            os_boots: [now.os_boot_id].into(),
            key_ids: [keys.id.clone()].into(),
            key_history: keys.fingerprints().into(),
            keys,
            rules: HashMap::new(),
            prepared: HashMap::new(),
            grants: HashMap::new(),
            decisions: HashSet::new(),
            invocations: HashSet::new(),
            approvals: HashSet::new(),
            reservation_ids: HashSet::new(),
            nonces: HashSet::new(),
        })
    }

    fn sample(&mut self, clock: &impl Clock) -> Result<ClockSample> {
        let now = match clock.sample() {
            Ok(now) => now,
            Err(_) => {
                self.invalidate_all(EndReason::ClockUntrusted);
                return Err(Failure::Time);
            }
        };
        if now.os_boot_id != self.high_water.os_boot_id {
            self.boot_fenced = true;
            self.invalidate_all(EndReason::EpochChanged);
        }
        Ok(now)
    }

    fn now(&mut self, clock: &impl Clock) -> Result<ClockSample> {
        let now = self.sample(clock)?;
        if self.boot_fenced {
            return Err(Failure::Epoch);
        }
        if now.utc < self.high_water.utc || now.monotonic_ms < self.high_water.monotonic_ms {
            self.invalidate_all(EndReason::ClockUntrusted);
            return Err(Failure::Time);
        }
        self.high_water = now;
        Ok(now)
    }

    pub fn advance_epochs(&mut self, role: Role, next: Epochs, clock: &impl Clock) -> Result<()> {
        require_role(role, Role::Core)?;
        let now = self.sample(clock)?;
        next.validate()?;
        require_ids(&[now.os_boot_id])?;
        let reboot = now.os_boot_id != self.high_water.os_boot_id;
        let new_core = next.core != self.epochs.core;
        if (reboot && (!new_core || self.os_boots.contains(&now.os_boot_id)))
            || (self.boot_fenced && !reboot)
            || (new_core && self.core_epochs.contains(&next.core))
            || next.policy.0 < self.epochs.policy.0
            || next.grant.0 < self.epochs.grant.0
            || next == self.epochs
        {
            return Err(Failure::Epoch);
        }
        if now.utc < self.high_water.utc
            || (!reboot && now.monotonic_ms < self.high_water.monotonic_ms)
        {
            self.invalidate_all(EndReason::ClockUntrusted);
            return Err(Failure::Time);
        }
        self.invalidate_all(EndReason::EpochChanged);
        self.core_epochs.insert(next.core);
        self.os_boots.insert(now.os_boot_id);
        self.epochs = next;
        self.high_water = now;
        self.boot_fenced = false;
        Ok(())
    }

    pub fn rotate_keys(
        &mut self,
        role: Role,
        keys: SyntheticKeys,
        grant: GrantEpoch,
        clock: &impl Clock,
    ) -> Result<()> {
        require_role(role, Role::Core)?;
        self.now(clock)?;
        let next = Epochs {
            grant,
            ..self.epochs
        };
        next.validate()?;
        let fingerprints = keys.fingerprints();
        if keys.generation <= self.keys.generation
            || grant.0 <= self.epochs.grant.0
            || self.key_ids.contains(&keys.id)
            || fingerprints
                .iter()
                .any(|hash| self.key_history.contains(hash))
        {
            return Err(Failure::Key);
        }
        self.invalidate_all(EndReason::KeyRotated);
        self.key_ids.insert(keys.id.clone());
        self.key_history.extend(fingerprints);
        self.keys = keys;
        self.epochs = next;
        Ok(())
    }

    fn invalidate_all(&mut self, reason: EndReason) {
        for grant in self
            .grants
            .values_mut()
            .filter(|grant| grant.status.state.is_open())
        {
            grant
                .transition(GrantState::Invalidated, Some(reason))
                .expect("open grant");
        }
        for prepared in self.prepared.values_mut() {
            prepared.snapshot = None;
        }
    }

    pub fn prepare(
        &mut self,
        role: Role,
        observed: &Observation<'_>,
        clock: &impl Clock,
    ) -> Result<()> {
        require_role(role, Role::Broker)?;
        let now = self.now(clock)?;
        let binding = observed.preparation;
        if !binding.deadline.live(now) {
            return Err(Failure::Expired);
        }
        let snapshot = Snapshot::read(&self.schemas, &self.keys, observed, now.os_boot_id)?;
        if self.prepared.contains_key(&binding.id)
            || self.decisions.contains(&observed.decision.id)
            || self.invocations.contains(&snapshot.invocation)
        {
            return Err(Failure::Duplicate);
        }
        self.invocations.insert(snapshot.invocation);
        self.decisions.insert(observed.decision.id);
        self.prepared.insert(
            binding.id,
            Prepared {
                binding,
                decision: observed.decision,
                snapshot: Some(snapshot),
                grant_id: None,
                epochs: self.epochs,
            },
        );
        Ok(())
    }

    pub fn prepared_status(&self, id: Uuid) -> Option<PreparedStatus> {
        self.prepared.get(&id).map(|item| {
            if item.snapshot.is_some() {
                PreparedStatus::Ready
            } else {
                PreparedStatus::Retired
            }
        })
    }

    pub fn retire_prepared(&mut self, role: Role, id: Uuid) -> Result<()> {
        require_role(role, Role::Core)?;
        self.retire(id, GrantState::Invalidated, EndReason::BindingChanged)
    }

    fn retire(&mut self, id: Uuid, state: GrantState, reason: EndReason) -> Result<()> {
        let item = self.prepared.get_mut(&id).ok_or(Failure::UnknownRecord)?;
        item.snapshot = None;
        if let Some(grant) = item.grant_id.and_then(|id| self.grants.get_mut(&id))
            && grant.status.state.is_open()
        {
            grant.transition(state, Some(reason))?;
        }
        Ok(())
    }

    /// A mismatched peer cannot retire someone else's preparation. Observed changes
    /// from the matching fixture route retire it permanently.
    fn recheck(&mut self, id: Uuid, observed: &Observation<'_>, now: ClockSample) -> Result<()> {
        let prepared = self.prepared.get(&id).ok_or(Failure::UnknownRecord)?;
        if observed.preparation.id != id || observed.preparation.peer != prepared.binding.peer {
            return Err(Failure::InvalidBinding);
        }
        if !prepared.binding.deadline.live(now) {
            self.retire(id, GrantState::Expired, EndReason::Deadline)?;
            return Err(Failure::Expired);
        }
        if prepared.epochs != self.epochs {
            return Err(Failure::Epoch);
        }
        let snapshot = prepared.snapshot.as_ref().ok_or(Failure::InvalidState)?;
        let check = observed.decision.eligible().and_then(|()| {
            if observed.decision != prepared.decision || observed.preparation != prepared.binding {
                return Err(Failure::InvalidBinding);
            }
            snapshot.matches(&self.keys, observed)
        });
        if check.is_err() {
            self.retire(id, GrantState::Invalidated, EndReason::BindingChanged)?;
        }
        check
    }

    pub fn approve_rule(
        &mut self,
        role: Role,
        spec: RuleSpec,
        observed: &Observation<'_>,
        clock: &impl Clock,
    ) -> Result<RuleIntent> {
        require_role(role, Role::Ui)?;
        let now = self.now(clock)?;
        let prepared_id = observed.preparation.id;
        self.recheck(prepared_id, observed, now)?;
        Ok(RuleIntent {
            spec,
            prepared_id,
            epochs: self.epochs,
        })
    }

    pub fn register_rule(
        &mut self,
        role: Role,
        intent: RuleIntent,
        clock: &impl Clock,
    ) -> Result<()> {
        require_role(role, Role::Core)?;
        let now = self.now(clock)?;
        let RuleIntent {
            spec,
            prepared_id,
            epochs,
        } = intent;
        if epochs != self.epochs {
            return Err(Failure::Epoch);
        }
        require_ids(&[spec.reference.id])?;
        if spec.reference.version == 0
            || spec.purpose.trim().is_empty()
            || spec.purpose.chars().count() > 256
            || spec.max_operations == 0
            || spec.valid_from >= spec.expires_at
            || now.utc >= spec.expires_at
            || self.rules.contains_key(&spec.reference.id)
        {
            return Err(Failure::RuleUnavailable);
        }
        let prepared = self
            .prepared
            .get(&prepared_id)
            .ok_or(Failure::UnknownRecord)?;
        if !prepared.binding.deadline.live(now) {
            return Err(Failure::Expired);
        }
        let snapshot = prepared.snapshot.as_ref().ok_or(Failure::InvalidState)?;
        let scope = snapshot.scope();
        self.rules.insert(
            spec.reference.id,
            ParentRule {
                spec,
                scope,
                epochs: self.epochs,
                consumed_operations: 0,
                revoked: false,
            },
        );
        Ok(())
    }

    pub fn revoke_rule(&mut self, role: Role, reference: RuleRef) -> Result<()> {
        require_role(role, Role::Core)?;
        let rule = self
            .rules
            .get_mut(&reference.id)
            .ok_or(Failure::UnknownRecord)?;
        if rule.spec.reference != reference || rule.revoked {
            return Err(Failure::RuleUnavailable);
        }
        rule.revoked = true;
        for grant in self.grants.values_mut() {
            if grant.approval.parent == Some(reference) && grant.status.state.is_open() {
                grant.transition(GrantState::Revoked, Some(EndReason::ParentRevoke))?;
                self.prepared
                    .get_mut(&grant.prepared_id)
                    .expect("grant has preparation")
                    .snapshot = None;
            }
        }
        Ok(())
    }

    pub fn approve(
        &mut self,
        role: Role,
        approval: ManualApproval,
        observed: &Observation<'_>,
        clock: &impl Clock,
    ) -> Result<()> {
        require_role(role, Role::Ui)?;
        let now = self.now(clock)?;
        require_ids(&[approval.approval_id, approval.grant_id])?;
        let id = observed.preparation.id;
        self.recheck(id, observed, now)?;
        let prepared = &self.prepared[&id];
        if !approval.deadline.live(now) || !approval.deadline.within(prepared.binding.deadline) {
            return Err(Failure::Expired);
        }
        if prepared.grant_id.is_some()
            || self.grants.contains_key(&approval.grant_id)
            || self.approvals.contains(&approval.approval_id)
        {
            return Err(Failure::Duplicate);
        }
        let snapshot = prepared.snapshot.as_ref().expect("rechecked");
        self.check_parent(approval.parent, snapshot, now, true)?;
        let intent = intent_payload(&snapshot.semantic, self.epochs, approval.deadline.utc);
        let mut grant = Grant {
            status: GrantStatus {
                state: GrantState::Created,
                reason: None,
            },
            approval,
            prepared_id: id,
            decision_id: prepared.decision.id,
            invocation_id: snapshot.invocation,
            epochs: self.epochs,
            intent_mac: self.keys.sign(Domain::Intent, &intent)?,
            reservation: None,
            unconfirmed: false,
        };
        grant.transition(GrantState::Pending, None)?;
        self.prepared.get_mut(&id).expect("rechecked").grant_id = Some(approval.grant_id);
        self.approvals.insert(approval.approval_id);
        self.grants.insert(approval.grant_id, grant);
        Ok(())
    }

    fn check_parent(
        &self,
        reference: Option<RuleRef>,
        snapshot: &Snapshot,
        now: ClockSample,
        check_budget: bool,
    ) -> Result<()> {
        if let Some(reference) = reference {
            let rule = self
                .rules
                .get(&reference.id)
                .ok_or(Failure::RuleUnavailable)?;
            rule.active(reference, self.epochs, now)?;
            if rule.scope != snapshot.scope() {
                return Err(Failure::InvalidBinding);
            }
            if check_budget && rule.consumed_operations >= rule.spec.max_operations {
                return Err(Failure::Exhausted);
            }
        }
        Ok(())
    }

    pub fn reserve(
        &mut self,
        role: Role,
        request: ReserveRequest,
        observed: &Observation<'_>,
        clock: &impl Clock,
    ) -> Result<ReserveResult> {
        require_role(role, Role::Core)?;
        let now = self.now(clock)?;
        let grant = self
            .grants
            .get(&request.grant_id)
            .ok_or(Failure::UnknownRecord)?;
        if grant.unconfirmed {
            return Err(Failure::UnknownCommit);
        }
        if grant.status.state != GrantState::Pending {
            return Err(Failure::InvalidState);
        }
        let id = grant.prepared_id;
        self.recheck(id, observed, now)?;
        let grant = &self.grants[&request.grant_id];
        if !grant.approval.deadline.live(now) {
            self.retire(id, GrantState::Expired, EndReason::Deadline)?;
            return Err(Failure::Expired);
        }
        require_ids(&[request.reservation_id])?;
        if !(1..=60_000).contains(&request.ttl_ms) {
            return Err(Failure::Time);
        }
        if request.nonce == [0; 32] {
            return Err(Failure::InvalidBinding);
        }
        let nonce_hash = Sha256::digest(request.nonce).into();
        if self.nonces.contains(&nonce_hash) {
            return Err(Failure::NonceReuse);
        }
        if self.reservation_ids.contains(&request.reservation_id) {
            return Err(Failure::Duplicate);
        }
        let prepared = &self.prepared[&id];
        let snapshot = prepared.snapshot.as_ref().expect("rechecked");
        let parent = grant.approval.parent;
        self.check_parent(parent, snapshot, now, true)?;
        let deadline = Deadline::after(now, u64::from(request.ttl_ms))?;
        if !deadline.within(grant.approval.deadline)
            || parent.is_some_and(|rule| deadline.utc > self.rules[&rule.id].spec.expires_at)
        {
            return Err(Failure::Expired);
        }
        let intent = intent_payload(&snapshot.semantic, self.epochs, grant.approval.deadline.utc);
        self.keys
            .verify(Domain::Intent, &intent, &grant.intent_mac)?;
        if request.outcome == CommitObservation::Rejected {
            self.retire(id, GrantState::Failed, EndReason::ReserveRejected)?;
            return Ok(ReserveResult::Rejected);
        }
        let mut reservation = Reservation {
            id: request.reservation_id,
            grant_id: request.grant_id,
            decision_id: grant.decision_id,
            approval_id: grant.approval.approval_id,
            preparation: prepared.binding,
            invocation_id: grant.invocation_id,
            semantic_mac: snapshot.semantic.clone(),
            operation_mac: String::new(),
            manifest: snapshot.manifest,
            nonce_hash,
            key_id: self.keys.id.clone(),
            key_generation: self.keys.generation,
            os_boot_id: now.os_boot_id,
            epochs: grant.epochs,
            deadline,
            ttl_ms: request.ttl_ms,
        };
        let mut payload = reservation.permit_payload(snapshot, &crypto::hex(&request.nonce))?;
        reservation.operation_mac = self
            .keys
            .sign(Domain::Operation, &operation_payload(&payload))?;
        payload["operation_digest"] = json!(reservation.operation_mac);
        payload["mac"] = json!(self.keys.sign(Domain::Permit, &payload)?);
        self.schemas
            .validate("permit-v2", &payload)
            .map_err(|_| Failure::InvalidWire)?;

        // No fallible validation follows. The one mutable owner records all indexes
        // and the charge together before returning any response.
        let grant = self
            .grants
            .get_mut(&request.grant_id)
            .expect("pending grant");
        grant
            .transition(GrantState::Reserved, None)
            .expect("checked pending grant");
        grant.reservation = Some(reservation);
        self.nonces.insert(nonce_hash);
        self.reservation_ids.insert(request.reservation_id);
        if let Some(parent) = parent {
            self.rules
                .get_mut(&parent.id)
                .expect("checked parent")
                .consumed_operations += 1;
        }
        if request.outcome == CommitObservation::ConfirmationLost {
            grant.unconfirmed = true;
            self.prepared.get_mut(&id).expect("rechecked").snapshot = None;
            return Ok(ReserveResult::Unknown);
        }
        Ok(ReserveResult::Issued(Permit(payload)))
    }

    pub fn consume(
        &mut self,
        role: Role,
        wire: &[u8],
        observed: &Observation<'_>,
        outcome: CommitObservation,
        clock: &impl Clock,
    ) -> Result<SimulatedConsumption> {
        require_role(role, Role::Broker)?;
        let now = self.now(clock)?;
        let mut payload = parse_json(wire).map_err(|_| Failure::InvalidWire)?;
        self.schemas
            .validate("permit-v2", &payload)
            .map_err(|_| Failure::InvalidWire)?;
        if text(&payload, "key_id")? != self.keys.id {
            return Err(Failure::Key);
        }
        let mac = payload
            .as_object_mut()
            .expect("permit object")
            .remove("mac")
            .expect("required mac");
        self.keys.verify(
            Domain::Permit,
            &payload,
            mac.as_str().ok_or(Failure::InvalidWire)?,
        )?;
        let id = parse_uuid(&payload, "grant_id")?;
        let grant = self.grants.get(&id).ok_or(Failure::UnknownRecord)?;
        if grant.unconfirmed {
            return Err(Failure::UnknownCommit);
        }
        if grant.status.state != GrantState::Reserved {
            return Err(Failure::InvalidState);
        }
        let prepared_id = grant.prepared_id;
        self.recheck(prepared_id, observed, now)?;
        let grant = &self.grants[&id];
        let reservation = grant.reservation.as_ref().expect("reserved grant");
        let snapshot = self.prepared[&prepared_id]
            .snapshot
            .as_ref()
            .expect("rechecked");
        if !reservation.deadline.live(now) || !grant.approval.deadline.live(now) {
            self.retire(prepared_id, GrantState::Expired, EndReason::Deadline)?;
            return Err(Failure::Expired);
        }
        if reservation.epochs != self.epochs
            || reservation.epochs != grant.epochs
            || reservation.os_boot_id != now.os_boot_id
            || reservation.key_generation != self.keys.generation
        {
            return Err(Failure::Epoch);
        }
        self.check_parent(grant.approval.parent, snapshot, now, false)?;
        if reservation.decision_id != observed.decision.id
            || reservation.approval_id != grant.approval.approval_id
            || reservation.preparation != observed.preparation
            || reservation.manifest != snapshot.manifest
            || reservation.invocation_id != snapshot.invocation
            || !self.nonces.contains(&reservation.nonce_hash)
        {
            return Err(Failure::InvalidBinding);
        }
        let nonce_text = text(&payload, "nonce")?;
        let nonce = crypto::decode_mac(nonce_text).map_err(|_| Failure::InvalidWire)?;
        let nonce_hash: [u8; 32] = Sha256::digest(nonce).into();
        let mut expected = reservation.permit_payload(snapshot, nonce_text)?;
        let submitted_operation = payload
            .as_object_mut()
            .expect("permit object")
            .remove("operation_digest")
            .expect("required operation digest");
        expected
            .as_object_mut()
            .expect("permit object")
            .remove("operation_digest");
        if nonce_hash != reservation.nonce_hash || payload != expected {
            return Err(Failure::InvalidBinding);
        }
        let operation = operation_payload(&payload);
        self.keys.verify(
            Domain::Operation,
            &operation,
            submitted_operation.as_str().ok_or(Failure::InvalidWire)?,
        )?;
        self.keys
            .verify(Domain::Operation, &operation, &reservation.operation_mac)?;
        if outcome == CommitObservation::Rejected {
            self.retire(prepared_id, GrantState::Failed, EndReason::ConsumeRejected)?;
            return Err(Failure::InvalidState);
        }
        let grant = self.grants.get_mut(&id).expect("checked reserved grant");
        grant.transition(GrantState::Consumed, None)?;
        self.prepared
            .get_mut(&prepared_id)
            .expect("rechecked")
            .snapshot = None;
        if outcome == CommitObservation::ConfirmationLost {
            grant.unconfirmed = true;
            return Err(Failure::UnknownCommit);
        }
        Ok(SimulatedConsumption::Recorded)
    }

    pub fn revoke_grant(&mut self, role: Role, id: Uuid) -> Result<()> {
        require_role(role, Role::Core)?;
        let grant = self.grants.get_mut(&id).ok_or(Failure::UnknownRecord)?;
        grant.transition(GrantState::Revoked, Some(EndReason::ManualRevoke))?;
        self.prepared
            .get_mut(&grant.prepared_id)
            .expect("grant has preparation")
            .snapshot = None;
        Ok(())
    }

    pub fn status(&self, id: Uuid) -> ReconcileResult {
        match self.grants.get(&id) {
            None => ReconcileResult::UnknownRecord,
            Some(grant) if grant.unconfirmed => ReconcileResult::Unconfirmed(grant.status),
            Some(grant) => ReconcileResult::Observed(grant.status),
        }
    }

    pub fn audit_mac(&self, role: Role, link: &AuditLink<'_>) -> Result<String> {
        require_role(role, Role::Core)?;
        self.keys.sign(Domain::Audit, &link.payload(&self.keys)?)
    }

    pub fn verify_audit_mac(&self, role: Role, link: &AuditLink<'_>, mac: &str) -> Result<()> {
        require_role(role, Role::Core)?;
        self.keys
            .verify(Domain::Audit, &link.payload(&self.keys)?, mac)
    }
}

impl Reservation {
    fn permit_payload(&self, snapshot: &Snapshot, nonce: &str) -> Result<Value> {
        let mut payload = snapshot.permit_identity()?;
        let reservation = json!({
            "schema_version": "2.0.0", "grant_id": self.grant_id.to_string(),
            "reservation_id": self.id.to_string(), "nonce": nonce, "key_id": self.key_id,
            "core_boot_epoch": self.epochs.core.0.to_string(), "broker_instance": self.preparation.peer.broker_instance.to_string(),
            "prepared_id": self.preparation.id.to_string(), "semantic_digest": self.semantic_mac,
            "operation_digest": self.operation_mac, "policy_epoch": self.epochs.policy.0,
            "grant_epoch": self.epochs.grant.0, "expires_at": self.deadline.utc.rfc3339(),
            "ttl_ms": self.ttl_ms, "max_uses": 1,
        });
        payload
            .as_object_mut()
            .expect("identity object")
            .extend(reservation.as_object().expect("reservation object").clone());
        Ok(payload)
    }
}
