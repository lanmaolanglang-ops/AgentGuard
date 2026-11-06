use crate::{
    Action, AuditKind, Binding, Bytes, Digest, Epoch, Error, Id, Ingest, Mutation, Observation,
    Outcome, Receipt, Result, State, db,
};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};
use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

const REJECTED_REQUEST_BUDGET: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Stage {
    BeforeAudit,
    BeforeCommit,
    AfterCommit,
}

#[cfg(test)]
type Hook = Box<dyn FnMut(Stage, &Connection) -> Result<()> + Send>;

#[derive(Default)]
struct Faults {
    #[cfg(test)]
    hook: Option<Hook>,
}

impl Faults {
    fn reach(&mut self, _stage: Stage, _db: &Connection) -> Result<()> {
        #[cfg(test)]
        if let Some(hook) = self.hook.as_mut() {
            return hook(_stage, _db);
        }
        Ok(())
    }
}

pub(crate) trait Clock: Send {
    fn read(&self) -> Result<(i64, i64)>;
}

struct ProcessClock(Instant);

impl Clock for ProcessClock {
    fn read(&self) -> Result<(i64, i64)> {
        let wall = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Clock)?;
        Ok((
            wall.as_millis().try_into().map_err(|_| Error::Clock)?,
            self.0
                .elapsed()
                .as_millis()
                .try_into()
                .map_err(|_| Error::Clock)?,
        ))
    }
}

struct Time {
    clock: Box<dyn Clock>,
    wall_high: i64,
    mono_high: i64,
}

impl Time {
    fn now(&mut self) -> Result<(i64, i64)> {
        let (wall, mono) = self.clock.read()?;
        if wall < 0 || mono < self.mono_high {
            return Err(Error::Clock);
        }
        self.wall_high = self.wall_high.max(wall);
        self.mono_high = mono;
        Ok((self.wall_high, mono))
    }
}

pub struct Ledger {
    connection: Connection,
    _lock: File,
    epoch: Epoch,
    time: Time,
    fenced: bool,
    rejected_requests: HashSet<Id>,
    faults: Faults,
}

impl Ledger {
    pub fn create(path: &Path, policy: u32, grant_epoch: u32) -> Result<Self> {
        Self::start(path, policy, grant_epoch, true)
    }

    pub fn reopen(path: &Path, policy: u32, grant_epoch: u32) -> Result<Self> {
        Self::start(path, policy, grant_epoch, false)
    }

    fn start(path: &Path, policy: u32, grant_epoch: u32, create: bool) -> Result<Self> {
        let root = db::fixture_dir(path)?;
        let lock = db::lock(&root)?;
        let path = root.join(db::DB);
        if create {
            if [
                "ledger.sqlite3-wal",
                "ledger.sqlite3-shm",
                "ledger.sqlite3-journal",
                "history.sqlite3",
                "history.pending",
            ]
            .iter()
            .any(|name| root.join(name).exists())
            {
                return Err(Error::Conflict);
            }
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|error| {
                    if error.kind() == std::io::ErrorKind::AlreadyExists {
                        Error::Conflict
                    } else {
                        Error::Storage
                    }
                })?;
        }
        let mut connection = db::connection(&path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        let prior = if create { 0 } else { db::version(&connection)? };
        if prior != 0 && db::epoch(&connection)?.1 {
            return Err(Error::Archived);
        }
        db::configure_writer(&connection)?;
        let epoch = Epoch {
            boot: Bytes(uuid::Uuid::new_v4().into_bytes()),
            policy,
            grant: grant_epoch,
        };
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        db::migrate(&tx, prior, epoch)?;
        let (stored, archived) = db::epoch(&tx)?;
        if archived || epoch.policy < stored.policy || epoch.grant < stored.grant {
            return Err(Error::Epoch);
        }
        db::integrity(&tx)?;
        let unfinished = tx
            .prepare("SELECT id,state FROM grants WHERE state IN ('created','pending','reserved')")?
            .query_map([], |row| {
                Ok((row.get::<_, Id>(0)?, row.get::<_, State>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (grant, previous) in unfinished {
            transition(&tx, grant, previous, State::Invalidated, None)?;
            insert_audit(
                &tx,
                grant,
                AuditKind::Transition,
                Some(previous),
                State::Invalidated,
                None,
            )?;
        }
        tx.execute(
            "UPDATE meta SET boot=?,policy=?,grant_epoch=? WHERE singleton=1",
            (epoch.boot, epoch.policy, epoch.grant),
        )?;
        tx.commit().map_err(|_| Error::UnknownCommit)?;
        Ok(Self {
            connection,
            _lock: lock,
            epoch,
            fenced: false,
            rejected_requests: HashSet::new(),
            faults: Faults::default(),
            time: Time {
                clock: Box::new(ProcessClock(Instant::now())),
                wall_high: 0,
                mono_high: 0,
            },
        })
    }

    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub fn apply(&mut self, mutation: Mutation) -> Result<Outcome> {
        if self.fenced {
            return Err(Error::Fenced);
        }
        let result = self.apply_inner(mutation);
        self.note_fault(&result);
        if result.is_err() && !self.fenced {
            self.rejected_requests.insert(mutation.request);
            if self.rejected_requests.len() >= REJECTED_REQUEST_BUDGET {
                self.fenced = true;
            }
        }
        result
    }

    fn apply_inner(&mut self, mutation: Mutation) -> Result<Outcome> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let fingerprint = mutation.fingerprint();
        if let Some((receipt, previous)) = db::find_receipt(&tx, mutation.request)? {
            return if previous == fingerprint {
                Ok(Outcome::Reconciled(receipt))
            } else {
                Err(Error::Conflict)
            };
        }
        if self.rejected_requests.contains(&mutation.request) {
            return Err(Error::Replay);
        }
        let (epoch, archived) = db::epoch(&tx)?;
        if archived || epoch != self.epoch || mutation.binding.epoch != epoch {
            return Err(Error::Epoch);
        }
        let (kind, previous, state) = mutate(&tx, mutation, &mut self.time)?;
        self.faults.reach(Stage::BeforeAudit, &tx)?;
        let receipt = insert_audit(
            &tx,
            mutation.binding.grant,
            kind,
            previous,
            state,
            Some((mutation.request, fingerprint)),
        )?;
        self.faults.reach(Stage::BeforeCommit, &tx)?;
        tx.commit().map_err(|_| Error::UnknownCommit)?;
        self.faults
            .reach(Stage::AfterCommit, &self.connection)
            .map_err(|_| Error::UnknownCommit)?;
        Ok(Outcome::Durable(receipt))
    }

    fn note_fault<T>(&mut self, result: &Result<T>) {
        if matches!(
            result,
            Err(Error::UnknownCommit
                | Error::Corrupt
                | Error::Sqlite(_)
                | Error::Clock
                | Error::Busy)
        ) {
            self.fenced = true;
        }
    }

    pub fn ingest(&mut self, observation: Observation) -> Result<Ingest> {
        if self.fenced {
            return Err(Error::Fenced);
        }
        let result = self.ingest_inner(observation);
        self.note_fault(&result);
        result
    }

    fn ingest_inner(&mut self, event: Observation) -> Result<Ingest> {
        if !(1..=9_007_199_254_740_991).contains(&event.sequence) {
            return Err(Error::Input);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if db::epoch(&tx)? != (self.epoch, false) {
            return Err(Error::Epoch);
        }
        let existing: Option<(i64, i64, Digest)> = tx.query_row(
            "SELECT ingest_seq,source_seq,digest FROM observations WHERE source=? AND event_id=?",
            (event.source, event.event),
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).optional()?;
        if let Some((sequence, old_sequence, old_digest)) = existing {
            return if old_sequence == event.sequence && old_digest == event.digest {
                Ok(Ingest::Duplicate { sequence })
            } else {
                Err(Error::Conflict)
            };
        }
        let inserted = tx.execute(
            "INSERT INTO observations(source,event_id,source_seq,digest) VALUES(?,?,?,?)
             ON CONFLICT(source,source_seq) DO NOTHING",
            (event.source, event.event, event.sequence, event.digest),
        )?;
        if inserted != 1 {
            return Err(Error::Conflict);
        }
        let sequence = tx.last_insert_rowid();
        tx.commit().map_err(|_| Error::UnknownCommit)?;
        Ok(Ingest::Durable { sequence })
    }

    pub fn backup_to(&self, archive: &Path) -> Result<()> {
        db::archive(&self.connection, archive)
    }

    #[cfg(test)]
    pub(crate) fn set_clock(&mut self, clock: Box<dyn Clock>) {
        self.time.clock = clock;
    }

    #[cfg(test)]
    pub(crate) fn set_hook(&mut self, hook: Hook) {
        self.faults.hook = Some(hook);
    }

    #[cfg(test)]
    pub(crate) fn db(&self) -> &Connection {
        &self.connection
    }
}

struct Grant {
    state: State,
    eligible: bool,
    nonce: Option<Digest>,
    deadline: Option<(i64, i64)>,
}

fn load(db: &Connection, binding: Binding) -> Result<Grant> {
    let (actual, state, eligible, wall, mono, nonce) = db
        .query_row(
            "SELECT d.id,d.input_digest,d.boot,d.policy,d.grant_epoch,g.state,d.eligible,
                g.wall_deadline,g.mono_deadline,n.hash
         FROM grants g JOIN decisions d ON d.id=g.decision
         LEFT JOIN nonces n ON n.grant_id=g.id WHERE g.id=?",
            [binding.grant],
            |r| {
                Ok((
                    Binding {
                        grant: binding.grant,
                        decision: r.get(0)?,
                        input: r.get(1)?,
                        epoch: Epoch {
                            boot: r.get(2)?,
                            policy: r.get(3)?,
                            grant: r.get(4)?,
                        },
                    },
                    r.get(5)?,
                    r.get(6)?,
                    r.get::<_, Option<i64>>(7)?,
                    r.get::<_, Option<i64>>(8)?,
                    r.get(9)?,
                ))
            },
        )
        .optional()?
        .ok_or(Error::Binding)?;
    if actual != binding {
        return Err(Error::Binding);
    }
    Ok(Grant {
        state,
        eligible,
        nonce,
        deadline: wall.zip(mono),
    })
}

fn mutate(
    tx: &Transaction<'_>,
    mutation: Mutation,
    time: &mut Time,
) -> Result<(AuditKind, Option<State>, State)> {
    let b = mutation.binding;
    if let Action::Create { eligible } = mutation.action {
        if tx.query_row::<bool, _, _>(
            "SELECT EXISTS(SELECT 1 FROM grants WHERE id=?) OR EXISTS(SELECT 1 FROM decisions WHERE id=?)",
            (b.grant, b.decision), |r| r.get(0))?
        {
            return Err(Error::Conflict);
        }
        tx.execute(
            "INSERT INTO decisions VALUES(?,?,?,?,?,?)",
            (
                b.decision,
                b.input,
                b.epoch.boot,
                b.epoch.policy,
                b.epoch.grant,
                eligible,
            ),
        )?;
        tx.execute(
            "INSERT INTO grants(id,decision,state) VALUES(?,?,'created')",
            (b.grant, b.decision),
        )?;
        return Ok((AuditKind::Transition, None, State::Created));
    }

    let grant = load(tx, b)?;
    let mut deadline = None;
    let next = match mutation.action {
        Action::Create { .. } => unreachable!(),
        Action::Pend => State::Pending,
        Action::Reserve { nonce, ttl_ms } => {
            if grant.state != State::Pending {
                return Err(Error::Transition);
            }
            if !grant.eligible {
                return Err(Error::Ineligible);
            }
            if !(1..=60_000).contains(&ttl_ms) {
                return Err(Error::Input);
            }
            let (wall, mono) = time.now()?;
            let ttl = i64::from(ttl_ms);
            deadline = Some((
                wall.checked_add(ttl).ok_or(Error::Clock)?,
                mono.checked_add(ttl).ok_or(Error::Clock)?,
            ));
            if tx.execute(
                "INSERT INTO nonces VALUES(?,?) ON CONFLICT(hash) DO NOTHING",
                (nonce, b.grant),
            )? != 1
            {
                return Err(Error::Replay);
            }
            State::Reserved
        }
        Action::Consume { nonce } => {
            if grant.state == State::Consumed {
                return Err(Error::Replay);
            }
            if grant.state != State::Reserved {
                return Err(Error::Transition);
            }
            if !grant.eligible || grant.nonce != Some(nonce) {
                return Err(Error::Binding);
            }
            let (wall, mono) = time.now()?;
            let (wall_end, mono_end) = grant.deadline.ok_or(Error::Corrupt)?;
            if wall >= wall_end || mono >= mono_end {
                State::Expired
            } else {
                tx.execute("INSERT INTO execution VALUES(?,'unknown',0)", [b.grant])?;
                State::Consumed
            }
        }
        Action::Revoke if grant.state == State::Consumed => {
            if tx.execute("UPDATE execution SET cancel_requested=1 WHERE grant_id=? AND state='unknown' AND cancel_requested=0", [b.grant])? != 1 {
                return Err(Error::Transition);
            }
            return Ok((AuditKind::CancelRequested, None, State::Consumed));
        }
        Action::Revoke => State::Revoked,
        Action::Expire => {
            if grant.state == State::Reserved {
                let (wall, mono) = time.now()?;
                let (wall_end, mono_end) = grant.deadline.ok_or(Error::Corrupt)?;
                if wall < wall_end && mono < mono_end {
                    return Err(Error::Transition);
                }
            }
            State::Expired
        }
        Action::Invalidate => State::Invalidated,
        Action::FailPreparation => State::Failed,
        Action::WorkerFailed => {
            if grant.state != State::Consumed
                || tx.execute(
                    "UPDATE execution SET state='failed' WHERE grant_id=? AND state='unknown'",
                    [b.grant],
                )? != 1
            {
                return Err(Error::Transition);
            }
            return Ok((AuditKind::WorkerFailed, None, State::Consumed));
        }
    };
    transition(tx, b.grant, grant.state, next, deadline)?;
    Ok((AuditKind::Transition, Some(grant.state), next))
}

fn transition(
    tx: &Transaction<'_>,
    id: Id,
    previous: State,
    next: State,
    deadline: Option<(i64, i64)>,
) -> Result<()> {
    if !previous.permits(next) {
        return Err(Error::Transition);
    }
    let (wall, mono) = deadline.unzip();
    if tx.execute(
        "UPDATE grants SET state=?,wall_deadline=coalesce(?,wall_deadline),
         mono_deadline=coalesce(?,mono_deadline) WHERE id=? AND state=?",
        params![next.name(), wall, mono, id, previous.name()],
    )? != 1
    {
        return Err(Error::Transition);
    }
    Ok(())
}

fn insert_audit(
    tx: &Transaction<'_>,
    grant: Id,
    kind: AuditKind,
    previous: Option<State>,
    state: State,
    request: Option<(Id, Digest)>,
) -> Result<Receipt> {
    let (id, fingerprint) = request.unzip();
    tx.execute(
        "INSERT INTO audit(grant_id,kind,previous,state,request,fingerprint) VALUES(?,?,?,?,?,?)",
        params![
            grant,
            kind.name(),
            previous.map(State::name),
            state.name(),
            id,
            fingerprint
        ],
    )?;
    Ok(Receipt {
        sequence: tx.last_insert_rowid(),
        grant,
        state,
        kind,
    })
}
