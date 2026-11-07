//! Explicit S1 synthetic fixture mode on the G0 connection and Writer.
use super::{Faults, Ledger, ProcessClock, Time};
use crate::{Bytes, Digest, Epoch, Error, Id, Result, State, db};
use agentguard_typed_authorization as d;
use d::s_preflight::WriterVerifier;
pub use d::s_preflight::{
    ConsumeCandidate, PrecommitCandidate, preflight_s_consume, preflight_s_reserve,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use serde_json::json;
use sha2::{Digest as _, Sha256};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
    time::Instant,
};
use uuid::Uuid;

#[cfg(test)]
#[path = "s_tests.rs"]
mod tests;
#[path = "s_transaction.rs"]
mod transaction;

pub(super) fn typed<T>(value: d::Result<T>) -> Result<T> {
    value.map_err(|error| match error {
        d::Failure::Time => Error::Clock,
        d::Failure::Expired => Error::Transition,
        d::Failure::Epoch | d::Failure::Key => Error::Epoch,
        _ => Error::Binding,
    })
}
pub(super) fn id(value: Uuid) -> Id {
    Bytes(value.into_bytes())
}
pub(super) fn uuid(value: Id) -> Uuid {
    Uuid::from_bytes(value.0)
}
fn integer(value: u64) -> Result<i64> {
    i64::try_from(value).map_err(|_| Error::Input)
}
fn new_id() -> Id {
    id(Uuid::new_v4())
}
pub(super) fn digest(value: &[u8]) -> Digest {
    Bytes(Sha256::digest(value).into())
}

/// Caller must explicitly construct these test clocks. No OS clock claim is made.
#[derive(Clone)]
pub struct SyntheticClock(Arc<Mutex<Option<d::ClockSample>>>);
impl SyntheticClock {
    pub fn new(sample: d::ClockSample) -> Self {
        Self(Arc::new(Mutex::new(Some(sample))))
    }
    pub fn set(&self, sample: Option<d::ClockSample>) {
        *self.0.lock().expect("fixture clock") = sample;
    }
    fn sample(&self) -> Result<d::ClockSample> {
        self.0.lock().map_err(|_| Error::Clock)?.ok_or(Error::Clock)
    }
}

/// Finite, explicitly registered nonce fixture values. Raw values never reach SQL.
pub struct SyntheticNonces {
    values: VecDeque<[u8; 32]>,
    attempts: usize,
}
impl SyntheticNonces {
    pub fn new(values: Vec<[u8; 32]>) -> Self {
        Self {
            values: values.into(),
            attempts: 0,
        }
    }
    fn next(&mut self) -> Result<[u8; 32]> {
        self.attempts += 1;
        let value = self.values.pop_front().ok_or(Error::Fenced)?;
        if value == [0; 32] {
            return Err(Error::Fenced);
        }
        Ok(value)
    }
}

pub struct FixtureConfig {
    pub keys: d::SyntheticKeys,
    pub clock: SyntheticClock,
    pub nonces: SyntheticNonces,
    #[cfg(test)]
    startup_fault: Option<ActivationFault>,
}
#[cfg(test)]
#[derive(Clone, Copy)]
enum ActivationFault {
    AfterMarker,
    BeforeCommit,
    AfterCommit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Activation {
    pub sequence: i64,
    pub lineage: Id,
    pub core: Id,
    pub clock_domain: Id,
    pub os_boot: Id,
    pub generation: u64,
}

/// Synthetic UI intent has private construction and is consumed by registration.
pub struct UiApproval {
    actor: Id,
    event: Id,
    challenge: Id,
    fingerprint: Digest,
    deadline: d::Deadline,
}
impl UiApproval {
    pub fn confirm(
        role: d::Role,
        actor: Uuid,
        event: Uuid,
        challenge: Uuid,
        candidate: &PrecommitCandidate,
        deadline: d::Deadline,
    ) -> Result<Self> {
        if role != d::Role::Ui || [actor, event, challenge].iter().any(Uuid::is_nil) {
            return Err(Error::Scope);
        }
        Ok(Self {
            actor: id(actor),
            event: id(event),
            challenge: id(challenge),
            fingerprint: Bytes(candidate.request_fingerprint().map_err(|_| Error::Input)?),
            deadline,
        })
    }
}
pub struct RuleApproval {
    reference: d::RuleRef,
    event: Id,
    approved_utc: u64,
    expires_utc: u64,
    max_operations: u32,
}
impl RuleApproval {
    pub fn confirm(
        role: d::Role,
        reference: d::RuleRef,
        event: Uuid,
        approved_utc: u64,
        max_operations: u32,
        lifetime_ms: u64,
    ) -> Result<Self> {
        if role != d::Role::Ui
            || reference.id.is_nil()
            || event.is_nil()
            || reference.version == 0
            || max_operations != 3
            || lifetime_ms != 600_000
        {
            return Err(Error::Scope);
        }
        let expires_utc = approved_utc.checked_add(lifetime_ms).ok_or(Error::Clock)?;
        typed(d::UtcMillis::from_unix_millis(expires_utc))?;
        Ok(Self {
            reference,
            event: id(event),
            approved_utc,
            expires_utc,
            max_operations,
        })
    }
}

pub enum SCommand {
    Policy {
        candidate: PrecommitCandidate,
    },
    Register {
        candidate: PrecommitCandidate,
        approval: UiApproval,
    },
    Rule {
        candidate: PrecommitCandidate,
        approval: RuleApproval,
    },
    Reserve {
        candidate: PrecommitCandidate,
        reservation: Uuid,
    },
    Consume {
        candidate: ConsumeCandidate,
        launch: Uuid,
    },
    RevokeApproval {
        approval: Uuid,
    },
    RevokeRule {
        rule: Uuid,
    },
    InvalidatePrepared {
        prepared: Uuid,
    },
    Cancel {
        grant: Uuid,
    },
}
pub struct SRequest {
    pub request: Id,
    pub activation: Id,
    pub command: SCommand,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommitView {
    pub request: Id,
    pub sequence: i64,
    pub grant: Option<Id>,
    pub reservation: Option<Id>,
    pub launch: Option<Id>,
    pub activation: i64,
}
/// Only the original queue response can transfer the unforgeable response key.
pub struct DurableCommitReceipt {
    view: CommitView,
    slot: Option<Arc<()>>,
}
impl DurableCommitReceipt {
    pub fn view(&self) -> CommitView {
        self.view
    }
}
pub enum SOutcome {
    Fresh(DurableCommitReceipt),
    StatusOnly(CommitView),
}
pub enum SResponse {
    Permit(d::Permit),
    Launch(LaunchAuthorization),
}
/// Data-level launch registration only. This type has no Worker execution method.
pub struct LaunchAuthorization {
    metadata: LaunchMetadata,
}
impl LaunchAuthorization {
    pub fn metadata(&self) -> LaunchMetadata {
        self.metadata
    }
}
#[derive(Clone, Copy)]
pub struct LaunchMetadata {
    pub launch: Id,
    pub reservation: Id,
    pub grant: Id,
    pub peer: d::PeerBinding,
    pub prepared: Uuid,
    pub prepared_generation: u64,
    pub activation: Activation,
    pub deadline: d::Deadline,
}
struct Slot {
    grant: Id,
    peer: d::PeerBinding,
    deadline: d::Deadline,
    response: SResponse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaultPoint {
    Claim,
    Quota,
    G0Reservation,
    Nonce,
    Reservation,
    State,
    Execution,
    Launch,
    Facts,
    G0Audit,
    Audit,
    Receipt,
    BeforeCommit,
    CommitUnknown,
    AfterCommit,
    Egress,
}
#[cfg(test)]
type FaultHook = Box<dyn FnMut(FaultPoint, &Connection) -> Result<()> + Send>;

pub(crate) struct Runtime {
    activation: Activation,
    marker: PathBuf,
    _lineage_lock: File,
    verifier: WriterVerifier,
    clock: SyntheticClock,
    high: d::ClockSample,
    nonces: SyntheticNonces,
    slots: HashMap<Id, (Weak<()>, Slot)>,
    #[cfg(test)]
    hook: Option<FaultHook>,
}

impl Runtime {
    pub(crate) fn reap_abandoned_slots(&mut self) {
        self.slots.retain(|_, (owner, _)| owner.strong_count() > 0);
    }
    fn now(&mut self) -> Result<d::ClockSample> {
        let now = self.clock.sample()?;
        if id(now.os_boot_id) != self.activation.os_boot
            || now.utc < self.high.utc
            || now.monotonic_ms < self.high.monotonic_ms
            || now.monotonic_ms > i64::MAX as u64
        {
            return Err(Error::Clock);
        }
        self.high = now;
        Ok(now)
    }
    fn reach(&mut self, _point: FaultPoint, _db: &Connection) -> Result<()> {
        #[cfg(test)]
        if let Some(hook) = self.hook.as_mut() {
            hook(_point, _db)?;
        }
        Ok(())
    }
    pub(super) fn check_marker(&self) -> Result<()> {
        if fs::read(&self.marker).map_err(|_| Error::Fenced)? != marker_bytes(self.activation) {
            return Err(Error::Fenced);
        }
        Ok(())
    }
}

fn marker_bytes(a: Activation) -> Vec<u8> {
    format!(
        "AgentGuard S1 synthetic fence\n{}\n{}\n{}\n{}\n{}\n{}\n",
        uuid(a.lineage),
        a.sequence,
        uuid(a.core),
        uuid(a.clock_domain),
        uuid(a.os_boot),
        a.generation
    )
    .into_bytes()
}

impl Ledger {
    pub fn create_s(
        path: &Path,
        fence: &Path,
        policy: u32,
        grant: u32,
        config: FixtureConfig,
    ) -> Result<Self> {
        Self::create(path, policy, grant)?.enable_s(fence, config)
    }

    /// Build a new fixture from a read-only source snapshot. The source stays at v3.
    pub fn migrate_s_copy(
        source: &Path,
        destination: &Path,
        fence: &Path,
        config: FixtureConfig,
    ) -> Result<Self> {
        let source = db::connection(
            &db::fixture_dir(source)?.join(db::DB),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        source.execute_batch("PRAGMA query_only=ON")?;
        let snapshot = source.unchecked_transaction()?;
        if db::version(&snapshot)? != 3 || db::epoch(&snapshot)?.1 {
            return Err(Error::Version);
        }
        db::integrity(&snapshot)?;
        let epoch = db::epoch(&snapshot)?.0;
        let mut target = Self::create(destination, epoch.policy, epoch.grant)?;
        db::backup_snapshot(&snapshot, &mut target.connection)?;
        snapshot.commit()?;
        target.enable_s(fence, config)
    }

    fn enable_s(mut self, fence: &Path, config: FixtureConfig) -> Result<Self> {
        if db::version(&self.connection)? != 3 || self.s.is_some() {
            return Err(Error::Version);
        }
        self.activate_s(fence, config, true)?;
        Ok(self)
    }
    pub fn reopen_s(
        path: &Path,
        fence: &Path,
        policy: u32,
        grant: u32,
        config: FixtureConfig,
    ) -> Result<Self> {
        let root = db::fixture_dir(path)?;
        let lock = db::lock(&root)?;
        let connection = db::connection(&root.join(db::DB), OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        if db::recognized_version(&connection)? != 4 {
            return Err(Error::Version);
        }
        db::integrity(&connection)?;
        let (old, archived) = db::epoch(&connection)?;
        if archived {
            return Err(Error::Archived);
        }
        if policy < old.policy || grant <= old.grant {
            return Err(Error::Epoch);
        }
        let mut ledger = Self {
            connection,
            _lock: lock,
            epoch: Epoch {
                policy,
                grant,
                ..old
            },
            time: Time {
                clock: Box::new(ProcessClock(Instant::now())),
                wall_high: 0,
                mono_high: 0,
            },
            fenced: false,
            rejected_requests: HashSet::new(),
            faults: Faults::default(),
            s: None,
        };
        ledger.activate_s(fence, config, false)?;
        Ok(ledger)
    }
    fn activate_s(&mut self, fence: &Path, config: FixtureConfig, migrate: bool) -> Result<()> {
        let root = db::fixture_dir(fence)?;
        if root.join(db::DB).exists() {
            return Err(Error::Scope);
        }
        let lineage_lock = db::lock(&root)?;
        let marker = root.join("s-fence");
        if let Ok(metadata) = fs::symlink_metadata(&marker) {
            use std::os::windows::fs::MetadataExt;
            if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
                return Err(Error::Scope);
            }
        }
        db::integrity(&self.connection)?;
        let verifier = typed(WriterVerifier::new(config.keys))?;
        let now = config.clock.sample()?;
        if now.os_boot_id.is_nil()
            || now.monotonic_ms > i64::MAX as u64
            || verifier.generation() > i64::MAX as u64
        {
            return Err(Error::Clock);
        }
        let (sequence, lineage) = if migrate {
            if marker.exists() {
                return Err(Error::Fenced);
            }
            (1, new_id())
        } else {
            let prior = read_activation(&self.connection)?;
            if fs::read(&marker).map_err(|_| Error::Fenced)? != marker_bytes(prior)
                || verifier.generation() <= prior.generation
            {
                return Err(Error::Fenced);
            }
            for fingerprint in verifier.key_fingerprints() {
                if self.connection.query_row::<bool, _, _>(
                    "SELECT EXISTS(SELECT 1 FROM s_key_history WHERE fingerprint=?)",
                    [Bytes(fingerprint)],
                    |r| r.get(0),
                )? {
                    return Err(Error::Epoch);
                }
            }
            (
                prior.sequence.checked_add(1).ok_or(Error::Fenced)?,
                prior.lineage,
            )
        };
        db::configure_writer(&self.connection)?;
        let activation = Activation {
            sequence,
            lineage,
            core: new_id(),
            clock_domain: new_id(),
            os_boot: id(now.os_boot_id),
            generation: verifier.generation(),
        };
        // Advance the external fence before touching the activation transaction.
        // Any interrupted transition remains closed for independent investigation.
        let mut marker_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&marker)
            .map_err(|_| Error::Storage)?;
        marker_file
            .write_all(&marker_bytes(activation))
            .map_err(|_| Error::Storage)?;
        marker_file.sync_all().map_err(|_| Error::Storage)?;
        #[cfg(test)]
        if matches!(config.startup_fault, Some(ActivationFault::AfterMarker)) {
            return Err(Error::Storage);
        }
        let runtime = Runtime {
            activation,
            marker,
            _lineage_lock: lineage_lock,
            verifier,
            clock: config.clock,
            high: now,
            nonces: config.nonces,
            slots: HashMap::new(),
            #[cfg(test)]
            hook: None,
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if migrate {
            tx.execute_batch(db::V4)?;
            tx.pragma_update(None, "user_version", 4)?;
        }
        let epoch = Epoch {
            boot: activation.core,
            ..self.epoch
        };
        tx.execute(
            "INSERT INTO s_activation_history VALUES(?,?,?,?,?,?,?,?,?)",
            params![
                activation.sequence,
                activation.lineage,
                activation.core,
                activation.clock_domain,
                activation.os_boot,
                runtime.verifier.key_id(),
                integer(activation.generation)?,
                epoch.policy,
                epoch.grant
            ],
        )?;
        for fingerprint in runtime.verifier.key_fingerprints() {
            tx.execute(
                "INSERT INTO s_key_history VALUES(?,?)",
                (Bytes(fingerprint), activation.sequence),
            )?;
        }
        tx.execute("INSERT INTO s_meta VALUES(1,?) ON CONFLICT(singleton) DO UPDATE SET activation=excluded.activation",[activation.sequence])?;
        let grants = tx
            .prepare("SELECT id,state FROM grants WHERE state IN ('created','pending','reserved')")?
            .query_map([], |r| Ok((r.get::<_, Id>(0)?, r.get::<_, State>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (grant, state) in grants {
            super::transition(&tx, grant, state, State::Invalidated, None)?;
            let g0 = super::insert_audit(
                &tx,
                grant,
                crate::AuditKind::Transition,
                Some(state),
                State::Invalidated,
                None,
            )?;
            if owns_grant(&tx, grant)? {
                runtime.audit(&tx, "invalidate", Some(grant), Some(g0.sequence), None)?;
            }
        }
        tx.execute(
            "UPDATE meta SET boot=?,policy=?,grant_epoch=? WHERE singleton=1",
            (epoch.boot, epoch.policy, epoch.grant),
        )?;
        runtime.audit(&tx, "activation", None, None, None)?;
        if !migrate {
            let activation_audit: i64 =
                tx.query_row("SELECT max(seq) FROM s_audit", [], |r| r.get(0))?;
            tx.execute("INSERT INTO s_execution_facts(grant_id,kind,audit_seq) SELECT s.grant_id,'recovery_unknown',? FROM s_grants s JOIN execution e ON e.grant_id=s.grant_id WHERE e.state='unknown'",[activation_audit])?;
        }
        db::integrity(&tx)?;
        #[cfg(test)]
        if matches!(config.startup_fault, Some(ActivationFault::BeforeCommit)) {
            tx.rollback().map_err(|_| Error::UnknownCommit)?;
            return Err(Error::Storage);
        }
        tx.commit().map_err(|_| Error::UnknownCommit)?;
        #[cfg(test)]
        if matches!(config.startup_fault, Some(ActivationFault::AfterCommit)) {
            return Err(Error::UnknownCommit);
        }
        runtime.check_marker()?;
        self.epoch = epoch;
        self.s = Some(runtime);
        Ok(())
    }
    pub fn s_activation(&self) -> Result<Activation> {
        Ok(self.s.as_ref().ok_or(Error::Scope)?.activation)
    }

    pub(crate) fn apply_s(&mut self, request: SRequest) -> Result<SOutcome> {
        if self.fenced {
            return Err(Error::Fenced);
        }
        let mut runtime = self.s.take().ok_or(Error::Scope)?;
        let request_id = request.request;
        let result = runtime.execute(
            &mut self.connection,
            self.epoch,
            &self.rejected_requests,
            request,
        );
        if matches!(
            result,
            Err(Error::Fenced | Error::UnknownCommit | Error::Storage)
        ) {
            self.fenced = true;
        }
        self.note_fault(&result);
        if result.is_err() {
            self.rejected_requests.insert(request_id);
        }
        if self.rejected_requests.len() >= super::REJECTED_REQUEST_BUDGET {
            self.fenced = true;
        }
        if self.fenced {
            runtime.slots.clear();
        }
        self.s = Some(runtime);
        result
    }
    pub(crate) fn take_s(
        &mut self,
        receipt: DurableCommitReceipt,
        peer: d::PeerBinding,
    ) -> Result<SResponse> {
        let result = self.take_s_inner(receipt, peer);
        self.note_fault(&result);
        if matches!(result, Err(Error::Fenced | Error::Storage)) {
            self.fenced = true;
        }
        if self.fenced
            && let Some(runtime) = self.s.as_mut()
        {
            runtime.slots.clear();
        }
        result
    }
    fn take_s_inner(
        &mut self,
        receipt: DurableCommitReceipt,
        peer: d::PeerBinding,
    ) -> Result<SResponse> {
        let runtime = self.s.as_mut().ok_or(Error::Scope)?;
        let owner = receipt.slot.ok_or(Error::Replay)?;
        let (key, _) = runtime
            .slots
            .get(&receipt.view.request)
            .ok_or(Error::Replay)?;
        if !key.ptr_eq(&Arc::downgrade(&owner)) {
            return Err(Error::Replay);
        }
        let (_, slot) = runtime
            .slots
            .remove(&receipt.view.request)
            .ok_or(Error::Replay)?;
        if self.fenced || peer != slot.peer {
            return Err(Error::Fenced);
        }
        runtime.check_marker()?;
        let now = runtime.now()?;
        if now.utc >= slot.deadline.utc || now.monotonic_ms >= slot.deadline.monotonic_ms {
            return Err(Error::Clock);
        }
        let tx = self.connection.transaction()?;
        runtime.check_current_grant(&tx, slot.grant, now)?;
        db::domain_integrity(&tx)?;
        domain_integrity(&tx)?;
        runtime.verify_audits(&tx)?;
        tx.commit()?;
        runtime.reach(FaultPoint::Egress, &self.connection)?;
        Ok(slot.response)
    }
}

pub(crate) fn owns_grant(db: &Connection, grant: Id) -> Result<bool> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM s_grants WHERE grant_id=?)",
        [grant],
        |r| r.get(0),
    )?)
}
pub(crate) fn has_request(db: &Connection, request: Id) -> Result<bool> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM s_requests WHERE id=?)",
        [request],
        |r| r.get(0),
    )?)
}
fn read_activation(db: &Connection) -> Result<Activation> {
    Ok(db.query_row("SELECT a.seq,a.lineage,a.core,a.clock_domain,a.os_boot,a.generation FROM s_activation_history a JOIN s_meta m ON m.activation=a.seq",[],|r|Ok(Activation {sequence:r.get(0)?,lineage:r.get(1)?,core:r.get(2)?,clock_domain:r.get(3)?,os_boot:r.get(4)?,generation:r.get::<_,i64>(5)? as u64}))?)
}
pub(crate) fn domain_integrity(db: &Connection) -> Result<()> {
    if db.prepare("PRAGMA foreign_key_check")?.exists([])?
        || db.prepare("SELECT 1 FROM s_binding_faults")?.exists([])?
    {
        return Err(Error::Corrupt);
    }
    let activation = read_activation(db)?;
    if db::epoch(db)?.0.boot != activation.core {
        return Err(Error::Corrupt);
    }
    if db.query_row::<bool,_,_>("SELECT (SELECT activation FROM s_meta)!=(SELECT max(seq) FROM s_activation_history) OR EXISTS(SELECT 1 FROM s_activation_history a WHERE a.seq!=(SELECT count(*) FROM s_activation_history b WHERE b.seq<=a.seq) OR a.lineage!=(SELECT lineage FROM s_activation_history WHERE seq=1) OR (SELECT count(*) FROM s_key_history k WHERE k.activation=a.seq)!=6)",[],|r|r.get(0))? {return Err(Error::Corrupt);}
    transaction::verify_structure(db)?;
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconcileView {
    Committed {
        receipt: CommitView,
        current_state: Option<State>,
        execution_unknown: bool,
    },
    NotObserved,
    Unavailable,
    Unknown,
}

impl crate::Inspection {
    /// A single read-only snapshot. Caller supplies its exact authorized Grant scope.
    pub fn reconcile_s(&self, request: Id, grant: Id) -> ReconcileView {
        let query = || -> Result<ReconcileView> {
            let tx = self.0.unchecked_transaction()?;
            if db::recognized_version(&tx)? != 4 {
                return Ok(ReconcileView::Unavailable);
            }
            db::integrity(&tx)?;
            let Some(receipt) = transaction::find_request(&tx, request)?.map(|r| r.0) else {
                return Ok(ReconcileView::NotObserved);
            };
            if receipt.grant != Some(grant) {
                return Ok(ReconcileView::Unavailable);
            }
            let current_state = tx
                .query_row("SELECT state FROM grants WHERE id=?", [grant], |r| r.get(0))
                .optional()?;
            let unknown = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM execution WHERE grant_id=? AND state='unknown')",
                [grant],
                |r| r.get(0),
            )?;
            tx.commit()?;
            Ok(ReconcileView::Committed {
                receipt,
                current_state,
                execution_unknown: unknown,
            })
        };
        match query() {
            Ok(view) => view,
            Err(Error::Corrupt) => ReconcileView::Unknown,
            Err(_) => ReconcileView::Unavailable,
        }
    }
}
