use crate::crypto::{digest, intent_payload, operation_payload, secret_payload};
use crate::*;
use agentguard_protocol::crypto::{self, Domain};
use agentguard_protocol::schema::Schemas;
use serde_json::{Value, json};
use std::cell::Cell;
use uuid::Uuid;

const NOW: u64 = 1_793_865_600_000; // 2026-11-05 08:00 UTC
const KEY_ID: &str = "fixture-secret";

fn id(value: u128) -> Uuid {
    Uuid::from_u128(0x0000_0000_0000_4000_8000_0000_0000_0000 | value)
}

fn utc(value: u64) -> UtcMillis {
    UtcMillis::from_unix_millis(value).unwrap()
}

fn keys(name: &str, generation: u64, start: u8) -> SyntheticKeys {
    SyntheticKeys::new(
        name.into(),
        generation,
        std::array::from_fn(|i| [start + i as u8; 32]),
    )
    .unwrap()
}

fn epochs() -> Epochs {
    Epochs {
        policy: PolicyEpoch(1),
        grant: GrantEpoch(2),
        core: CoreEpoch(id(0x602)),
    }
}

struct FixtureClock(Cell<Result<ClockSample>>);

impl Clock for FixtureClock {
    fn sample(&self) -> Result<ClockSample> {
        self.0.get()
    }
}

fn fixture(id: &str) -> Value {
    let rows: Vec<Value> =
        serde_json::from_str(include_str!("../../../fixtures/p0.1.json")).unwrap();
    rows.into_iter().find(|row| row["id"] == id).unwrap()["value"].clone()
}

fn encode(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn virtual_operation() -> Value {
    let mut op = fixture("OP01");
    op["handle_manifest"][0]["object_identity_digest"] =
        json!(crypto::hex(&digest(&op["targets"][0]).unwrap()));
    op
}

struct Setup {
    core: SyntheticCore,
    clock: FixtureClock,
    operation: Vec<u8>,
    preparation: Preparation,
    decision: Decision,
    approval: ManualApproval,
}

impl Setup {
    fn new() -> Self {
        let start = ClockSample {
            os_boot_id: id(0x500),
            utc: utc(NOW),
            monotonic_ms: 1000,
        };
        let clock = FixtureClock(Cell::new(Ok(start)));
        let core = SyntheticCore::new(epochs(), keys(KEY_ID, 1, 1), &clock).unwrap();
        Self {
            core,
            clock,
            operation: encode(&virtual_operation()),
            preparation: Preparation {
                id: id(0x604),
                peer: PeerBinding {
                    connection: id(0x606),
                    broker_instance: id(0x603),
                    worker_instance: id(0x605),
                },
                generation: 1,
                deadline: Deadline::after(start, 120_000).unwrap(),
            },
            decision: Decision {
                id: id(0x610),
                hard_deny: false,
                native_permission: NativePermission::Allow,
                identity: SafetyEvidence::Known,
                safety: SafetyEvidence::Known,
            },
            approval: ManualApproval {
                approval_id: id(0x611),
                grant_id: id(0x600),
                parent: None,
                deadline: Deadline::after(start, 60_000).unwrap(),
            },
        }
    }

    fn observation(&self) -> Observation<'_> {
        Observation {
            preparation: self.preparation,
            decision: self.decision,
            operation_json: &self.operation,
            secret_kinds: &[],
        }
    }

    fn request(&self) -> ReserveRequest {
        ReserveRequest {
            grant_id: self.approval.grant_id,
            reservation_id: id(0x601),
            nonce: [0xbb; 32],
            ttl_ms: 30_000,
            outcome: CommitObservation::Confirmed,
        }
    }

    fn prepare(&mut self) -> Result<()> {
        let Self {
            core,
            clock,
            operation,
            preparation,
            decision,
            ..
        } = self;
        core.prepare(
            Role::Broker,
            &Observation {
                preparation: *preparation,
                decision: *decision,
                operation_json: operation,
                secret_kinds: &[],
            },
            clock,
        )
    }

    fn approve(&mut self, role: Role) -> Result<()> {
        let Self {
            core,
            clock,
            operation,
            preparation,
            decision,
            approval,
        } = self;
        core.approve(
            role,
            *approval,
            &Observation {
                preparation: *preparation,
                decision: *decision,
                operation_json: operation,
                secret_kinds: &[],
            },
            clock,
        )
    }

    fn pending(&mut self) {
        self.prepare().unwrap();
        self.approve(Role::Ui).unwrap();
    }

    fn reserve(&mut self, request: ReserveRequest) -> Result<ReserveResult> {
        let Self {
            core,
            clock,
            operation,
            preparation,
            decision,
            ..
        } = self;
        core.reserve(
            Role::Core,
            request,
            &Observation {
                preparation: *preparation,
                decision: *decision,
                operation_json: operation,
                secret_kinds: &[],
            },
            clock,
        )
    }

    fn issue(&mut self) -> Vec<u8> {
        match self.reserve(self.request()).unwrap() {
            ReserveResult::Issued(permit) => permit.into_wire(),
            _ => panic!("expected synthetic Permit"),
        }
    }

    fn consume(&mut self, wire: &[u8], outcome: CommitObservation) -> Result<SimulatedConsumption> {
        let Self {
            core,
            clock,
            operation,
            preparation,
            decision,
            ..
        } = self;
        core.consume(
            Role::Broker,
            wire,
            &Observation {
                preparation: *preparation,
                decision: *decision,
                operation_json: operation,
                secret_kinds: &[],
            },
            outcome,
            clock,
        )
    }

    fn status(&self, state: GrantState, reason: Option<EndReason>) {
        assert_eq!(
            self.core.status(self.approval.grant_id),
            ReconcileResult::Observed(GrantStatus { state, reason })
        );
    }

    fn tick(&self, wall: u64, monotonic_ms: u64) {
        self.clock.0.set(Ok(ClockSample {
            os_boot_id: id(0x500),
            utc: utc(wall),
            monotonic_ms,
        }));
    }

    fn register_rule(&mut self, max_operations: u32) -> RuleRef {
        let reference = RuleRef {
            id: id(0x630),
            version: 7,
        };
        let spec = RuleSpec {
            reference,
            purpose: "合成读取".into(),
            valid_from: utc(NOW),
            expires_at: utc(NOW + 80_000),
            max_operations,
        };
        let Self {
            core,
            clock,
            operation,
            preparation,
            decision,
            ..
        } = self;
        let intent = core
            .approve_rule(
                Role::Ui,
                spec,
                &Observation {
                    preparation: *preparation,
                    decision: *decision,
                    operation_json: operation,
                    secret_kinds: &[],
                },
                clock,
            )
            .unwrap();
        core.register_rule(Role::Core, intent, clock).unwrap();
        reference
    }

    fn next_invocation(&mut self) {
        let mut operation: Value = serde_json::from_slice(&self.operation).unwrap();
        let next = id(0x703);
        operation["invocation_id"] = json!(next.to_string());
        self.operation = encode(&operation);
        self.preparation.id = id(0x634);
        self.decision.id = id(0x635);
        self.approval.grant_id = id(0x636);
        self.approval.approval_id = id(0x637);
    }
}

fn confirmed() -> CommitObservation {
    CommitObservation::Confirmed
}

#[test]
fn six_composed_macs_match_independent_references() {
    let mut s = Setup::new();
    s.pending();
    let wire = s.issue();
    let permit: Value = serde_json::from_slice(&wire).unwrap();
    let expected: Value = serde_json::from_str(include_str!("../fixture-macs.json")).unwrap();
    let keys = keys(KEY_ID, 1, 1);
    let secret = secret_payload("/arguments/項~1目~0", SecretKind::Utf8, "é🔒").unwrap();
    assert_eq!(
        keys.sign(Domain::Secret, &secret).unwrap(),
        expected["secret-v2"]
    );
    assert_eq!(permit["semantic_digest"], expected["semantic-v2"]);
    let intent = intent_payload(
        permit["semantic_digest"].as_str().unwrap(),
        epochs(),
        s.approval.deadline.utc,
    );
    assert_eq!(
        keys.sign(Domain::Intent, &intent).unwrap(),
        expected["intent-v2"]
    );
    assert_eq!(permit["operation_digest"], expected["operation-v2"]);
    assert_eq!(permit["mac"], expected["permit-v2"]);
    let link = AuditLink {
        segment_id: id(0x998),
        sequence: 1,
        previous_mac: None,
        event_digest: redacted_event_digest(br#"{"z":null,"a":1}"#).unwrap(),
        key_id: KEY_ID,
        key_generation: 1,
    };
    assert_eq!(
        s.core.audit_mac(Role::Core, &link).unwrap(),
        expected["audit-v2"]
    );
    assert_eq!(
        s.consume(&wire, confirmed()),
        Ok(SimulatedConsumption::Recorded)
    );
}

#[test]
fn frozen_grant_schema_agrees_on_all_64_transition_pairs() {
    use GrantState::*;
    let states = [
        (Created, "created"),
        (Pending, "pending"),
        (Reserved, "reserved"),
        (Consumed, "consumed"),
        (Revoked, "revoked"),
        (Expired, "expired"),
        (Invalidated, "invalidated"),
        (Failed, "failed"),
    ];
    let schemas = Schemas::new().unwrap();
    let mut legal = 0;
    for (before, previous_state) in states {
        for (after, state) in states {
            let mut wire = fixture(&format!("GR-{state}"));
            wire["previous_state"] = json!(previous_state);
            let schema_accepts = schemas.validate("grant-v2", &wire).is_ok();
            assert_eq!(
                before.permits(after),
                schema_accepts,
                "{before:?} -> {after:?}"
            );
            legal += u32::from(schema_accepts);
        }
    }
    assert_eq!(legal, 15);
    assert!(schemas.validate("grant-v2", &fixture("GR-created")).is_ok());
}

#[test]
fn manual_issue_consume_once_and_no_lookup_ticket() {
    let mut s = Setup::new();
    s.prepare().unwrap();
    assert_eq!(
        s.core.prepared_status(s.preparation.id),
        Some(PreparedStatus::Ready)
    );
    assert!(matches!(
        s.reserve(s.request()),
        Err(Failure::UnknownRecord)
    ));
    s.approve(Role::Ui).unwrap();
    s.status(GrantState::Pending, None);
    let wire = s.issue();
    s.status(GrantState::Reserved, None);
    assert!(matches!(s.reserve(s.request()), Err(Failure::InvalidState)));
    assert_eq!(
        s.consume(&wire, confirmed()),
        Ok(SimulatedConsumption::Recorded)
    );
    s.status(GrantState::Consumed, None);
    assert_eq!(
        s.core.prepared_status(s.preparation.id),
        Some(PreparedStatus::Retired)
    );
    assert_eq!(s.consume(&wire, confirmed()), Err(Failure::InvalidState));
    assert_eq!(
        s.core.revoke_grant(Role::Core, s.approval.grant_id),
        Err(Failure::InvalidState)
    );
    let outcome = ExecutionResult::ReportedFailure;
    assert_eq!(outcome, ExecutionResult::ReportedFailure);
    s.status(GrantState::Consumed, None);
}

#[test]
fn role_checks_and_rule_is_not_automatic_approval() {
    let mut s = Setup::new();
    let mut other_core = SyntheticCore::new(epochs(), keys(KEY_ID, 1, 1), &s.clock).unwrap();
    for role in [
        Role::Core,
        Role::Ui,
        Role::Adapter,
        Role::Worker,
        Role::Observer,
    ] {
        assert_eq!(
            other_core.prepare(role, &s.observation(), &s.clock),
            Err(Failure::WrongRole)
        );
    }
    s.prepare().unwrap();
    s.approval.parent = Some(s.register_rule(1));
    for role in [
        Role::Core,
        Role::Adapter,
        Role::Broker,
        Role::Worker,
        Role::Observer,
    ] {
        assert_eq!(s.approve(role), Err(Failure::WrongRole));
    }
    assert!(matches!(
        s.reserve(s.request()),
        Err(Failure::UnknownRecord)
    ));
    s.approve(Role::Ui).unwrap();
    let wire = s.issue();
    for role in [
        Role::Core,
        Role::Ui,
        Role::Adapter,
        Role::Worker,
        Role::Observer,
    ] {
        assert_eq!(
            other_core.consume(role, &wire, &s.observation(), confirmed(), &s.clock),
            Err(Failure::WrongRole)
        );
    }
    assert_eq!(
        s.consume(&wire, confirmed()),
        Ok(SimulatedConsumption::Recorded)
    );
}

#[test]
fn rule_budget_is_rechecked_at_reserve_with_two_pending_grants() {
    let mut s = Setup::new();
    s.prepare().unwrap();
    s.approval.parent = Some(s.register_rule(1));
    s.approve(Role::Ui).unwrap();
    let first = (s.preparation, s.decision, s.approval, s.operation.clone());
    s.next_invocation();
    s.pending();
    let second = (s.preparation, s.decision, s.approval, s.operation.clone());
    (s.preparation, s.decision, s.approval, s.operation) = first;
    let wire = s.issue();
    assert_eq!(
        s.consume(&wire, confirmed()),
        Ok(SimulatedConsumption::Recorded)
    );
    (s.preparation, s.decision, s.approval, s.operation) = second;
    let mut request = s.request();
    request.reservation_id = id(0x640);
    request.nonce = [0x44; 32];
    assert!(matches!(s.reserve(request), Err(Failure::Exhausted)));
    s.status(GrantState::Pending, None);
}

#[test]
fn rule_version_and_exact_scope_and_revoke() {
    let mut s = Setup::new();
    s.prepare().unwrap();
    let reference = s.register_rule(2);
    s.approval.parent = Some(RuleRef {
        version: 6,
        ..reference
    });
    assert_eq!(s.approve(Role::Ui), Err(Failure::RuleUnavailable));
    s.approval.parent = Some(reference);
    s.approve(Role::Ui).unwrap();
    let wire = s.issue();
    s.core.revoke_rule(Role::Core, reference).unwrap();
    s.status(GrantState::Revoked, Some(EndReason::ParentRevoke));
    assert_eq!(s.consume(&wire, confirmed()), Err(Failure::InvalidState));
    assert_eq!(
        s.core.revoke_rule(Role::Core, reference),
        Err(Failure::RuleUnavailable)
    );

    let mut scope = Setup::new();
    scope.prepare().unwrap();
    scope.approval.parent = Some(scope.register_rule(2));
    scope.next_invocation();
    let mut value: Value = serde_json::from_slice(&scope.operation).unwrap();
    value["provider_id"] = json!("other-synthetic-provider");
    scope.operation = encode(&value);
    scope.prepare().unwrap();
    assert_eq!(scope.approve(Role::Ui), Err(Failure::InvalidBinding));
}

#[test]
fn revoke_or_reject_does_not_refund_or_reissue() {
    let mut s = Setup::new();
    s.prepare().unwrap();
    s.approval.parent = Some(s.register_rule(1));
    s.approve(Role::Ui).unwrap();
    let wire = s.issue();
    s.core
        .revoke_grant(Role::Core, s.approval.grant_id)
        .unwrap();
    assert_eq!(
        s.core.revoke_grant(Role::Core, s.approval.grant_id),
        Err(Failure::InvalidState)
    );
    assert_eq!(s.consume(&wire, confirmed()), Err(Failure::InvalidState));
    s.next_invocation();
    s.prepare().unwrap();
    assert_eq!(s.approve(Role::Ui), Err(Failure::Exhausted));

    let mut rejected = Setup::new();
    rejected.pending();
    let mut request = rejected.request();
    request.outcome = CommitObservation::Rejected;
    assert!(matches!(
        rejected.reserve(request),
        Ok(ReserveResult::Rejected)
    ));
    rejected.status(GrantState::Failed, Some(EndReason::ReserveRejected));
    assert!(matches!(
        rejected.reserve(rejected.request()),
        Err(Failure::InvalidState)
    ));

    let mut rejected_consume = Setup::new();
    rejected_consume.pending();
    let permit = rejected_consume.issue();
    assert_eq!(
        rejected_consume.consume(&permit, CommitObservation::Rejected),
        Err(Failure::InvalidState)
    );
    rejected_consume.status(GrantState::Failed, Some(EndReason::ConsumeRejected));
    assert_eq!(
        rejected_consume.consume(&permit, confirmed()),
        Err(Failure::InvalidState)
    );
}

#[test]
fn unknown_commits_are_lookup_only_and_keep_nonce_and_quota() {
    let mut s = Setup::new();
    s.prepare().unwrap();
    s.approval.parent = Some(s.register_rule(1));
    s.approve(Role::Ui).unwrap();
    let mut request = s.request();
    request.outcome = CommitObservation::ConfirmationLost;
    assert!(matches!(s.reserve(request), Ok(ReserveResult::Unknown)));
    assert_eq!(
        s.core.status(s.approval.grant_id),
        ReconcileResult::Unconfirmed(GrantStatus {
            state: GrantState::Reserved,
            reason: None
        })
    );
    assert!(matches!(
        s.reserve(s.request()),
        Err(Failure::UnknownCommit)
    ));
    s.next_invocation();
    s.prepare().unwrap();
    assert_eq!(s.approve(Role::Ui), Err(Failure::Exhausted));
    s.approval.parent = None;
    s.approve(Role::Ui).unwrap();
    let mut request = s.request();
    request.reservation_id = id(0x642);
    assert!(matches!(s.reserve(request), Err(Failure::NonceReuse)));

    let mut lost = Setup::new();
    lost.pending();
    let wire = lost.issue();
    assert_eq!(
        lost.consume(&wire, CommitObservation::ConfirmationLost),
        Err(Failure::UnknownCommit)
    );
    assert_eq!(
        lost.consume(&wire, confirmed()),
        Err(Failure::UnknownCommit)
    );
    assert_eq!(
        lost.core.status(lost.approval.grant_id),
        ReconcileResult::Unconfirmed(GrantStatus {
            state: GrantState::Consumed,
            reason: None
        })
    );
    let mut fresh = Setup::new();
    assert_eq!(
        fresh.consume(&wire, confirmed()),
        Err(Failure::UnknownRecord)
    );
}

#[test]
fn nonce_collision_is_atomic_and_never_reclaims_terminal_nonce() {
    let mut s = Setup::new();
    s.pending();
    let wire = s.issue();
    assert_eq!(
        s.consume(&wire, confirmed()),
        Ok(SimulatedConsumption::Recorded)
    );
    s.next_invocation();
    s.prepare().unwrap();
    s.approval.parent = Some(s.register_rule(1));
    s.approve(Role::Ui).unwrap();
    let mut request = s.request();
    request.reservation_id = id(0x642);
    assert!(matches!(s.reserve(request), Err(Failure::NonceReuse)));
    s.status(GrantState::Pending, None);
    let mut request = s.request();
    request.nonce = [0xcc; 32];
    assert!(matches!(s.reserve(request), Err(Failure::Duplicate)));
    let mut request = s.request();
    request.nonce = [0xcc; 32];
    request.reservation_id = id(0x642);
    assert!(matches!(s.reserve(request), Ok(ReserveResult::Issued(_))));
}

#[test]
fn wire_schema_and_parser_reject_before_consuming() {
    let mut s = Setup::new();
    s.pending();
    let wire = s.issue();
    let baseline: Value = serde_json::from_slice(&wire).unwrap();
    let mut malformed = Vec::new();
    for (key, value) in [
        ("unused", json!(true)),
        ("ttl_ms", json!("30000")),
        ("ttl_ms", json!(0)),
        ("ttl_ms", json!(60_001)),
        ("policy_epoch", json!(9_007_199_254_740_992_u64)),
        ("grant_epoch", Value::Null),
        ("max_uses", json!(2)),
        ("max_uses", json!(true)),
        ("mac", json!("A".repeat(64))),
        ("mac", json!("a".repeat(63))),
        ("expires_at", json!("tomorrow")),
    ] {
        let mut value_wire = baseline.clone();
        value_wire[key] = value;
        malformed.push(encode(&value_wire));
    }
    let mut missing = baseline.clone();
    missing.as_object_mut().unwrap().remove("nonce");
    malformed.push(encode(&missing));
    let as_text = std::str::from_utf8(&wire).unwrap();
    malformed.push(format!(r#"{{"ttl\u005fms":30000,{}"#, &as_text[1..]).into_bytes());
    for invalid_number in ["3e4", "30000.0", "-0"] {
        malformed.push(
            as_text
                .replace(
                    r#""ttl_ms":30000"#,
                    &format!(r#""ttl_ms":{invalid_number}"#),
                )
                .into_bytes(),
        );
    }
    malformed.push(vec![b' '; 1_048_577]);
    let mut invalid_utf8 = wire.clone();
    invalid_utf8.insert(0, 0xff);
    malformed.push(invalid_utf8);
    malformed.push(as_text.replace(KEY_ID, r"\ud800").into_bytes());
    for malformed in malformed {
        assert_eq!(
            s.consume(&malformed, confirmed()),
            Err(Failure::InvalidWire)
        );
        s.status(GrantState::Reserved, None);
    }
    let mut reordered: Vec<_> = baseline.as_object().unwrap().iter().collect();
    reordered.reverse();
    let wire = format!(
        "{{{}}}",
        reordered
            .iter()
            .map(|(key, value)| format!("{}:{value}", serde_json::to_string(key).unwrap()))
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_eq!(
        s.consume(wire.as_bytes(), confirmed()),
        Ok(SimulatedConsumption::Recorded)
    );
}

#[test]
fn correct_permit_and_operation_macs_cannot_override_stored_binding() {
    let mut s = Setup::new();
    s.pending();
    let wire = s.issue();
    let baseline: Value = serde_json::from_slice(&wire).unwrap();
    let keys = keys(KEY_ID, 1, 1);
    let mut incorrect = baseline.clone();
    incorrect["mac"] = json!("0".repeat(64));
    assert_eq!(
        s.consume(&encode(&incorrect), confirmed()),
        Err(Failure::Mac)
    );
    let mut wrong_key = baseline.clone();
    wrong_key["key_id"] = json!("unknown");
    assert_eq!(
        s.consume(&encode(&wrong_key), confirmed()),
        Err(Failure::Key)
    );
    let mut wrong_domain = baseline.clone();
    wrong_domain.as_object_mut().unwrap().remove("mac");
    wrong_domain["mac"] = json!(keys.sign(Domain::Operation, &wrong_domain).unwrap());
    assert_eq!(
        s.consume(&encode(&wrong_domain), confirmed()),
        Err(Failure::Mac)
    );
    for (key, value) in [
        ("agent_id", json!("A02")),
        ("nonce", json!("cc".repeat(32))),
        ("policy_epoch", json!(2)),
        ("grant_epoch", json!(3)),
        ("core_boot_epoch", json!(id(0x990).to_string())),
        ("reservation_id", json!(id(0x991).to_string())),
        ("broker_instance", json!(id(0x992).to_string())),
        ("prepared_id", json!(id(0x993).to_string())),
        ("invocation_id", json!(id(0x994).to_string())),
        ("session_id", json!(id(0x995).to_string())),
        ("expires_at", json!("2026-11-05T08:00:59.000Z")),
        ("ttl_ms", json!(30_001)),
        ("runtime_digest", json!("a".repeat(64))),
        ("adapter_digest", json!("b".repeat(64))),
        ("requestor_principal_digest", json!("c".repeat(64))),
        ("executor_principal_digest", json!("d".repeat(64))),
        ("semantic_digest", json!("e".repeat(64))),
    ] {
        let mut signed = baseline.clone();
        signed.as_object_mut().unwrap().remove("mac");
        signed[key] = value;
        signed["operation_digest"] = json!(
            keys.sign(Domain::Operation, &operation_payload(&signed))
                .unwrap()
        );
        signed["mac"] = json!(keys.sign(Domain::Permit, &signed).unwrap());
        assert_eq!(
            s.consume(&encode(&signed), confirmed()),
            Err(Failure::InvalidBinding),
            "{key}"
        );
        s.status(GrantState::Reserved, None);
    }
    let mut wrong_operation_mac = baseline.clone();
    wrong_operation_mac.as_object_mut().unwrap().remove("mac");
    wrong_operation_mac["operation_digest"] = json!("0".repeat(64));
    wrong_operation_mac["mac"] = json!(keys.sign(Domain::Permit, &wrong_operation_mac).unwrap());
    assert_eq!(
        s.consume(&encode(&wrong_operation_mac), confirmed()),
        Err(Failure::Mac)
    );
    assert_eq!(
        s.consume(&wire, confirmed()),
        Ok(SimulatedConsumption::Recorded)
    );
}

#[test]
fn prepared_snapshot_mutations_permanently_retire_grant() {
    for (pointer, changed) in [
        ("/agent_id", json!("A02")),
        ("/runtime/version", json!("fixture-2")),
        ("/provider_id", json!("different")),
        ("/provider_digest", json!("0".repeat(64))),
        ("/adapter_id", json!("different")),
        ("/adapter_digest", json!("0".repeat(64))),
        ("/config_digest", json!("0".repeat(64))),
        ("/requestor_principal/integrity_level", json!("medium")),
        (
            "/executor_principal/windows_sid",
            json!("S-1-5-21-100-200-300-1002"),
        ),
        ("/session_id", json!(id(0x800).to_string())),
        ("/recipe", json!("file.write.v1")),
        ("/targets/0/path", json!(r"D:\GuardLab\other.txt")),
        ("/invocation_id", json!(id(0x801).to_string())),
        ("/arguments/max_bytes", json!(8192)),
        ("/working_directory/file_id", json!("dir2")),
        ("/handle_manifest/0/granted_access", json!(1)),
        ("/environment_digest", json!("0".repeat(64))),
        ("/stdin_digest", json!("0".repeat(64))),
        ("/input_digest", json!("0".repeat(64))),
        ("/script_digest", json!("0".repeat(64))),
        ("/executable_digest", json!("0".repeat(64))),
    ] {
        let mut s = Setup::new();
        s.pending();
        let wire = s.issue();
        let original = s.operation.clone();
        let mut change: Value = serde_json::from_slice(&original).unwrap();
        *change.pointer_mut(pointer).unwrap() = changed;
        s.operation = encode(&change);
        assert_eq!(
            s.consume(&wire, confirmed()),
            Err(Failure::InvalidBinding),
            "{pointer}"
        );
        s.status(GrantState::Invalidated, Some(EndReason::BindingChanged));
        s.operation = original;
        assert_eq!(
            s.consume(&wire, confirmed()),
            Err(Failure::InvalidState),
            "{pointer}"
        );
    }
}

#[test]
fn prepared_route_replay_is_denied_and_trusted_retirement_cannot_be_reverted() {
    let mut s = Setup::new();
    s.pending();
    let wire = s.issue();
    let original = s.preparation;
    for changed in [
        Preparation {
            id: id(0x910),
            ..original
        },
        Preparation {
            peer: PeerBinding {
                connection: id(0x911),
                ..original.peer
            },
            ..original
        },
        Preparation {
            peer: PeerBinding {
                broker_instance: id(0x912),
                ..original.peer
            },
            ..original
        },
        Preparation {
            peer: PeerBinding {
                worker_instance: id(0x913),
                ..original.peer
            },
            ..original
        },
    ] {
        s.preparation = changed;
        assert_eq!(s.consume(&wire, confirmed()), Err(Failure::InvalidBinding));
        s.status(GrantState::Reserved, None);
    }
    s.preparation = original;
    s.core.retire_prepared(Role::Core, original.id).unwrap();
    assert_eq!(s.consume(&wire, confirmed()), Err(Failure::InvalidState));
    assert_eq!(s.prepare(), Err(Failure::Duplicate));
}

#[test]
fn prepared_generation_deadline_and_decision_changes_invalidate_at_reserve() {
    for change in 0..3 {
        let mut s = Setup::new();
        s.pending();
        match change {
            0 => s.preparation.generation += 1,
            1 => s.preparation.deadline.monotonic_ms += 1,
            _ => s.decision.id = id(0x914),
        }
        assert!(matches!(
            s.reserve(s.request()),
            Err(Failure::InvalidBinding)
        ));
        s.status(GrantState::Invalidated, Some(EndReason::BindingChanged));
    }
}

#[test]
fn deny_and_unknown_fail_closed_at_preparation_and_redemption() {
    for (decision_change, error) in [
        (0, Failure::HardDeny),
        (1, Failure::NativeDeny),
        (2, Failure::UnknownEvidence),
        (3, Failure::UnknownEvidence),
        (4, Failure::UnknownEvidence),
    ] {
        let change = |s: &mut Setup| match decision_change {
            0 => s.decision.hard_deny = true,
            1 => s.decision.native_permission = NativePermission::Deny,
            2 => s.decision.native_permission = NativePermission::Unknown,
            3 => s.decision.identity = SafetyEvidence::Unknown,
            _ => s.decision.safety = SafetyEvidence::Unknown,
        };
        let mut denied = Setup::new();
        change(&mut denied);
        assert_eq!(denied.prepare(), Err(error));
        assert_eq!(
            denied.core.status(denied.approval.grant_id),
            ReconcileResult::UnknownRecord
        );
        let mut issued = Setup::new();
        issued.pending();
        let wire = issued.issue();
        change(&mut issued);
        assert_eq!(issued.consume(&wire, confirmed()), Err(error));
        issued.status(GrantState::Invalidated, Some(EndReason::BindingChanged));
    }
    let mut s = Setup::new();
    let mut op = virtual_operation();
    op["executor_principal"]["integrity_level"] = json!("unknown");
    s.operation = encode(&op);
    assert_eq!(s.prepare(), Err(Failure::UnknownEvidence));
    op["executor_principal"]["process_instance"]["os_boot_id"] = json!(id(0x501).to_string());
    op["executor_principal"]["integrity_level"] = json!("low");
    s.operation = encode(&op);
    assert_eq!(s.prepare(), Err(Failure::UnknownEvidence));
    op.as_object_mut().unwrap().remove("executor_principal");
    s.operation = encode(&op);
    assert_eq!(s.prepare(), Err(Failure::InvalidWire));
}

#[test]
fn utc_monotonic_rollback_and_clock_unavailability_never_extend_tickets() {
    for (wall, mono) in [(NOW + 30_000, 1001), (NOW + 1, 31_000)] {
        let mut s = Setup::new();
        s.pending();
        let wire = s.issue();
        s.tick(wall, mono);
        assert_eq!(s.consume(&wire, confirmed()), Err(Failure::Expired));
        s.status(GrantState::Expired, Some(EndReason::Deadline));
        assert_eq!(s.consume(&wire, confirmed()), Err(Failure::InvalidState));
    }
    for change in 0..3 {
        let mut s = Setup::new();
        s.pending();
        let wire = s.issue();
        match change {
            0 => s.tick(NOW - 1, 1001),
            1 => s.tick(NOW + 1, 999),
            _ => s.clock.0.set(Err(Failure::Time)),
        }
        assert_eq!(s.consume(&wire, confirmed()), Err(Failure::Time));
        s.status(GrantState::Invalidated, Some(EndReason::ClockUntrusted));
        s.tick(NOW + 1, 1001);
        assert_eq!(s.consume(&wire, confirmed()), Err(Failure::InvalidState));
    }
}

#[test]
fn ttl_limits_prepared_and_approval_deadlines() {
    let mut s = Setup::new();
    s.pending();
    for ttl in [0, 60_001] {
        let mut request = s.request();
        request.ttl_ms = ttl;
        assert!(matches!(s.reserve(request), Err(Failure::Time)));
    }
    let mut request = s.request();
    request.ttl_ms = 1;
    let wire = match s.reserve(request).unwrap() {
        ReserveResult::Issued(permit) => permit.into_wire(),
        _ => panic!("expected TTL=1 Permit"),
    };
    assert_eq!(
        s.consume(&wire, confirmed()),
        Ok(SimulatedConsumption::Recorded)
    );
    let mut upper = Setup::new();
    upper.pending();
    let mut request = upper.request();
    request.ttl_ms = 60_000;
    assert!(matches!(
        upper.reserve(request),
        Ok(ReserveResult::Issued(_))
    ));
    let mut late = Setup::new();
    late.pending();
    late.tick(NOW + 31_000, 32_000);
    assert!(matches!(
        late.reserve(late.request()),
        Err(Failure::Expired)
    ));
    late.status(GrantState::Pending, None);
    late.tick(NOW + 60_000, 61_000);
    assert!(matches!(
        late.reserve(late.request()),
        Err(Failure::Expired)
    ));
    late.status(GrantState::Expired, Some(EndReason::Deadline));
}

#[test]
fn separate_epochs_and_os_reboot_invalidate_all_pending_authority() {
    for next in [
        Epochs {
            policy: PolicyEpoch(2),
            ..epochs()
        },
        Epochs {
            grant: GrantEpoch(3),
            ..epochs()
        },
        Epochs {
            core: CoreEpoch(id(0x950)),
            ..epochs()
        },
    ] {
        let mut s = Setup::new();
        s.pending();
        let wire = s.issue();
        s.core.advance_epochs(Role::Core, next, &s.clock).unwrap();
        s.status(GrantState::Invalidated, Some(EndReason::EpochChanged));
        assert_eq!(s.consume(&wire, confirmed()), Err(Failure::InvalidState));
        assert_eq!(
            s.core.advance_epochs(Role::Core, epochs(), &s.clock),
            Err(Failure::Epoch)
        );
    }
    let mut s = Setup::new();
    s.pending();
    let wire = s.issue();
    s.clock.0.set(Ok(ClockSample {
        os_boot_id: id(0x999),
        utc: utc(NOW + 1),
        monotonic_ms: 1,
    }));
    assert_eq!(
        s.core.advance_epochs(Role::Core, epochs(), &s.clock),
        Err(Failure::Epoch)
    );
    s.status(GrantState::Invalidated, Some(EndReason::EpochChanged));
    assert_eq!(s.consume(&wire, confirmed()), Err(Failure::Epoch));
    s.tick(NOW + 1, 1001);
    s.next_invocation();
    assert_eq!(s.prepare(), Err(Failure::Epoch));
    s.clock.0.set(Ok(ClockSample {
        os_boot_id: id(0x999),
        utc: utc(NOW + 1),
        monotonic_ms: 1,
    }));
    let next = Epochs {
        core: CoreEpoch(id(0x950)),
        ..epochs()
    };
    s.core.advance_epochs(Role::Core, next, &s.clock).unwrap();
    assert_eq!(s.consume(&wire, confirmed()), Err(Failure::InvalidState));
    s.tick(NOW + 2, 2);
    let next = Epochs {
        core: CoreEpoch(id(0x951)),
        ..next
    };
    assert_eq!(
        s.core.advance_epochs(Role::Core, next, &s.clock),
        Err(Failure::Epoch)
    );
}

#[test]
fn rotation_requires_new_id_generation_epoch_and_material() {
    assert!(matches!(
        SyntheticKeys::new("test".into(), 1, [[1; 32]; 6]),
        Err(Failure::Key)
    ));
    let mut s = Setup::new();
    s.pending();
    let wire = s.issue();
    for (id, generation, material, epoch) in [
        (KEY_ID, 2, 10, 3),
        ("new", 1, 10, 3),
        ("new", 2, 1, 3),
        ("new", 2, 10, 2),
    ] {
        assert_eq!(
            s.core.rotate_keys(
                Role::Core,
                keys(id, generation, material),
                GrantEpoch(epoch),
                &s.clock
            ),
            Err(Failure::Key)
        );
        s.status(GrantState::Reserved, None);
    }
    s.core
        .rotate_keys(Role::Core, keys("new", 2, 10), GrantEpoch(3), &s.clock)
        .unwrap();
    s.status(GrantState::Invalidated, Some(EndReason::KeyRotated));
    assert_eq!(s.consume(&wire, confirmed()), Err(Failure::Key));
    s.next_invocation();
    assert_eq!(s.prepare(), Err(Failure::Key));
    let mut operation: Value = serde_json::from_slice(&s.operation).unwrap();
    operation["secret_parameters"]["key_id"] = json!("new");
    s.operation = encode(&operation);
    s.pending();
    let mut request = s.request();
    request.reservation_id = id(0x641);
    request.nonce = [0x31; 32];
    let fresh = match s.reserve(request).unwrap() {
        ReserveResult::Issued(permit) => permit.into_wire(),
        _ => panic!("expected new-generation Permit"),
    };
    assert_eq!(
        s.consume(&fresh, confirmed()),
        Ok(SimulatedConsumption::Recorded)
    );
}

#[test]
fn semantic_secret_shadow_preserves_unicode_null_and_other_fields() {
    let keys = keys(KEY_ID, 1, 1);
    let pointer = "/arguments/項~1目~0";
    let secret = "é🔒";
    let secret_mac = keys
        .sign(
            Domain::Secret,
            &secret_payload(pointer, SecretKind::Utf8, secret).unwrap(),
        )
        .unwrap();
    let op = json!({
        "invocation_id": id(1).to_string(), "command": null,
        "arguments": {"項/目~": secret, "other": null},
        "secret_parameters": {"key_id": KEY_ID, "entries": [{"json_pointer": pointer, "secret_mac": secret_mac}]},
    });
    let shadow = keys.semantic_payload(&op, &[SecretKind::Utf8]).unwrap();
    assert!(shadow.get("invocation_id").is_none());
    assert_eq!(
        shadow["arguments"]["項/目~"],
        json!({"key_id": KEY_ID, "secret_mac": secret_mac})
    );
    assert_eq!(shadow["arguments"]["other"], Value::Null);
    assert_eq!(shadow["command"], Value::Null);
    let semantic = keys.sign(Domain::Semantic, &shadow).unwrap();
    let mut changed = op.clone();
    changed["invocation_id"] = json!(id(2).to_string());
    let shadow = keys
        .semantic_payload(&changed, &[SecretKind::Utf8])
        .unwrap();
    keys.verify(Domain::Semantic, &shadow, &semantic).unwrap();
    changed["arguments"]["項/目~"] = json!("e\u{301}🔒");
    assert_eq!(
        keys.semantic_payload(&changed, &[SecretKind::Utf8]),
        Err(Failure::Mac)
    );
    let mut changed = op.clone();
    changed["arguments"]["other"] = json!("");
    let shadow = keys
        .semantic_payload(&changed, &[SecretKind::Utf8])
        .unwrap();
    assert_eq!(
        keys.verify(Domain::Semantic, &shadow, &semantic),
        Err(Failure::Mac)
    );
    let mut wrong_kind = op.clone();
    wrong_kind["arguments"]["項/目~"] = json!("YQ==");
    assert_eq!(
        keys.semantic_payload(&wrong_kind, &[SecretKind::Base64Bytes]),
        Err(Failure::Mac)
    );
}

#[test]
fn secret_validation_rejects_unregistered_and_ambiguous_pointers() {
    let keys = keys(KEY_ID, 1, 1);
    let entry = |pointer: &str, value: &str| {
        json!({
            "json_pointer": pointer,
            "secret_mac": keys.sign(Domain::Secret, &secret_payload(pointer, SecretKind::Utf8, value).unwrap()).unwrap()
        })
    };
    let mut op = json!({
        "invocation_id": "fixture-invocation", "command": "fixture text",
        "arguments": {"a":"x","b":"y","null":null}, "agent_id":"A01",
        "secret_parameters": {"key_id": KEY_ID, "entries": []},
    });
    for entries in [
        vec![entry("/arguments/a", "x"), entry("/arguments/a", "x")],
        vec![entry("/arguments/b", "y"), entry("/arguments/a", "x")],
        vec![entry("/arguments/a", "x"), entry("/arguments/a/child", "x")],
        vec![entry("/agent_id", "A01")],
        vec![entry("/command/0", "f")],
        vec![entry("/arguments/~2a", "x")],
        vec![entry("/arguments/null", "x")],
        vec![entry("/arguments/missing", "x")],
    ] {
        let kinds = vec![SecretKind::Utf8; entries.len()];
        op["secret_parameters"]["entries"] = json!(entries);
        assert_eq!(
            keys.semantic_payload(&op, &kinds),
            Err(Failure::InvalidBinding)
        );
    }
    op["secret_parameters"]["entries"] = json!([
        entry("/arguments/a", "x"),
        entry("/command", "fixture text")
    ]);
    assert!(keys.semantic_payload(&op, &[SecretKind::Utf8; 2]).is_ok());
    assert_eq!(keys.semantic_payload(&op, &[]), Err(Failure::Key));
    for value in ["YR==", "YQ=", "YWJ=", "____", "YQ==\n", "===="] {
        assert_eq!(
            secret_payload("/arguments/a", SecretKind::Base64Bytes, value),
            Err(Failure::InvalidBinding)
        );
    }
    for value in ["YQ==", "YWI=", "YWJj", "+/8=", ""] {
        assert!(secret_payload("/arguments/a", SecretKind::Base64Bytes, value).is_ok());
    }
}

#[test]
fn audit_mac_is_typed_and_bound_to_event_segment_previous_key_and_generation() {
    let s = Setup::new();
    let first = redacted_event_digest(br#"{"z":null,"a":1}"#).unwrap();
    let equivalent = redacted_event_digest(br#"{"a":1,"z":null}"#).unwrap();
    let changed = redacted_event_digest(br#"{"a":1,"z":""}"#).unwrap();
    let link = AuditLink {
        segment_id: id(0x998),
        sequence: 1,
        previous_mac: None,
        event_digest: first,
        key_id: KEY_ID,
        key_generation: 1,
    };
    let mac = s.core.audit_mac(Role::Core, &link).unwrap();
    s.core
        .verify_audit_mac(
            Role::Core,
            &AuditLink {
                event_digest: equivalent,
                ..link
            },
            &mac,
        )
        .unwrap();
    for change in [
        AuditLink {
            event_digest: changed,
            ..link
        },
        AuditLink {
            segment_id: id(0x999),
            ..link
        },
        AuditLink {
            sequence: 2,
            ..link
        },
        AuditLink {
            previous_mac: Some(&mac),
            ..link
        },
    ] {
        assert_eq!(
            s.core.verify_audit_mac(Role::Core, &change, &mac),
            Err(Failure::Mac)
        );
    }
    assert_eq!(
        s.core.verify_audit_mac(
            Role::Core,
            &AuditLink {
                key_generation: 2,
                ..link
            },
            &mac
        ),
        Err(Failure::Key)
    );
    assert_eq!(
        s.core.verify_audit_mac(
            Role::Core,
            &AuditLink {
                key_id: "other",
                ..link
            },
            &mac
        ),
        Err(Failure::Key)
    );
    assert_eq!(
        s.core.audit_mac(Role::Observer, &link),
        Err(Failure::WrongRole)
    );
    assert_eq!(
        s.core.audit_mac(
            Role::Core,
            &AuditLink {
                sequence: 9_007_199_254_740_992,
                ..link
            }
        ),
        Err(Failure::InvalidWire)
    );
    assert!(matches!(
        redacted_event_digest(br#"{"z":0,"z":0}"#),
        Err(Failure::InvalidWire)
    ));
}

#[test]
fn utc_fixture_formatter_handles_leap_century_and_limits() {
    for (millis, expected) in [
        (0, "1970-01-01T00:00:00.000Z"),
        (NOW, "2026-11-05T08:00:00.000Z"),
        (951_782_400_123, "2000-02-29T00:00:00.123Z"),
        (4_107_542_400_000, "2100-03-01T00:00:00.000Z"),
        (253_402_300_799_999, "9999-12-31T23:59:59.999Z"),
    ] {
        assert_eq!(utc(millis).rfc3339(), expected);
    }
    assert_eq!(
        UtcMillis::from_unix_millis(253_402_300_800_000),
        Err(Failure::Time)
    );
    assert_eq!(utc(253_402_300_799_999).plus(1), Err(Failure::Time));
}

#[test]
fn intent_payload_has_frozen_fields_and_one_claim() {
    let keys = keys(KEY_ID, 1, 1);
    let baseline = intent_payload(&"a".repeat(64), epochs(), utc(NOW + 1));
    assert_eq!(baseline.as_object().unwrap().len(), 6);
    assert_eq!(baseline["remaining_claims"], 1);
    let mac = keys.sign(Domain::Intent, &baseline).unwrap();
    keys.verify(Domain::Intent, &baseline, &mac).unwrap();
    for (key, value) in [
        ("remaining_claims", json!(2)),
        ("expires_at", json!(utc(NOW + 2).rfc3339())),
        ("policy_epoch", json!(4)),
        ("grant_epoch", json!(4)),
        ("core_boot_epoch", json!(id(0x955).to_string())),
        ("semantic_digest", json!("b".repeat(64))),
    ] {
        let mut changed = baseline.clone();
        changed[key] = value;
        assert_eq!(
            keys.verify(Domain::Intent, &changed, &mac),
            Err(Failure::Mac)
        );
    }
}
