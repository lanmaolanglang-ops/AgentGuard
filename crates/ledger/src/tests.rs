use super::*;
use crate::store::{Clock, Stage};
use rusqlite::OpenFlags;
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    sync::{
        Arc, Barrier,
        atomic::{AtomicI64, AtomicU64, Ordering},
        mpsc::sync_channel,
    },
    thread,
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(20);
const POLICY: u32 = 7;
const GRANT_EPOCH: u32 = 9;
const TARGET: u64 = 100;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("agentguard-ledger-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("fixture"), db::MARKER).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn create(&self) -> Ledger {
        Ledger::create(self.path(), POLICY, GRANT_EPOCH).unwrap()
    }

    fn reopen(&self) -> Ledger {
        Ledger::reopen(self.path(), POLICY, GRANT_EPOCH).unwrap()
    }

    fn inspection(&self) -> Inspection {
        let read = Inspection::open(self.path()).unwrap();
        read.verify().unwrap();
        read
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // The constructor created this UUID child exclusively for this test.
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn id(number: u64) -> Id {
    Bytes(u128::from(number).to_be_bytes())
}

fn request() -> Id {
    static NEXT: AtomicU64 = AtomicU64::new(10_000);
    id(NEXT.fetch_add(1, Ordering::Relaxed))
}

fn binding(epoch: Epoch, number: u64) -> Binding {
    Binding {
        grant: id(number),
        decision: id(number + 1_000),
        input: Bytes([17; 32]),
        epoch,
    }
}

fn mutate(binding: Binding, action: Action) -> Mutation {
    Mutation {
        request: request(),
        binding,
        action,
    }
}

fn durable(ledger: &mut Ledger, mutation: Mutation, state: State) -> Receipt {
    let Outcome::Durable(receipt) = ledger.apply(mutation).unwrap() else {
        panic!("expected a new durable record");
    };
    assert_eq!(receipt.state, state);
    receipt
}

fn create(ledger: &mut Ledger, number: u64) -> Binding {
    let b = binding(ledger.epoch(), number);
    durable(
        ledger,
        mutate(b, Action::Create { eligible: true }),
        State::Created,
    );
    b
}

fn pending(ledger: &mut Ledger, number: u64) -> Binding {
    let b = create(ledger, number);
    durable(ledger, mutate(b, Action::Pend), State::Pending);
    b
}

fn nonce(number: u64) -> Digest {
    let mut bytes = [0x92; 32];
    bytes[..8].copy_from_slice(&number.to_be_bytes());
    nonce_hash(&bytes)
}

fn reserve(ledger: &mut Ledger, b: Binding, nonce: Digest) {
    durable(
        ledger,
        mutate(
            b,
            Action::Reserve {
                nonce,
                ttl_ms: 60_000,
            },
        ),
        State::Reserved,
    );
}

#[derive(Clone)]
struct FakeClock(Arc<(AtomicI64, AtomicI64)>);

impl FakeClock {
    fn at(wall: i64, mono: i64) -> Self {
        Self(Arc::new((AtomicI64::new(wall), AtomicI64::new(mono))))
    }

    fn set(&self, wall: i64, mono: i64) {
        self.0.0.store(wall, Ordering::Relaxed);
        self.0.1.store(mono, Ordering::Relaxed);
    }
}

impl Clock for FakeClock {
    fn read(&self) -> Result<(i64, i64)> {
        Ok((
            self.0.0.load(Ordering::Relaxed),
            self.0.1.load(Ordering::Relaxed),
        ))
    }
}

#[test]
fn sqlite_build_omits_extension_loading_and_enforces_writer_pragmas() {
    let fixture = Fixture::new();
    let ledger = fixture.create();
    let actual: (String, String, i64, i64) = ledger.db().query_row(
        "SELECT sqlite_version(),sqlite_source_id(),
         sqlite_compileoption_used('OMIT_LOAD_EXTENSION'),sqlite_compileoption_used('ENABLE_LOAD_EXTENSION')",
        [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
    ).unwrap();
    assert_eq!((&*actual.0, actual.2, actual.3), ("3.54.1", 1, 0));
    let error = ledger
        .db()
        .prepare("SELECT load_extension('i05-nonexistent-fixture')")
        .err()
        .unwrap();
    assert!(
        error
            .to_string()
            .contains("no such function: load_extension")
    );
    assert_eq!(
        ledger
            .db()
            .query_row::<i64, _, _>("PRAGMA synchronous", [], |r| r.get(0))
            .unwrap(),
        2
    );
    assert_eq!(
        ledger
            .db()
            .query_row::<i64, _, _>("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap(),
        1
    );
    assert_eq!(
        ledger
            .db()
            .query_row::<String, _, _>("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap(),
        "wal"
    );
    assert_eq!(
        Ledger::reopen(fixture.path(), POLICY, GRANT_EPOCH).err(),
        Some(Error::Busy)
    );
    println!(
        "EVIDENCE SQLITE version={} source={} omit={} enable={} load_function=missing wal/full/fk=verified",
        actual.0, actual.1, actual.2, actual.3
    );
}

#[test]
fn at21_reconnect_restart_duplicates_late_events_and_gaps() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let b = create(&mut ledger, TARGET);
    let source = id(1001);
    let event = Observation {
        source,
        event: id(1201),
        sequence: 3,
        digest: Bytes([1; 32]),
    };
    assert_eq!(ledger.ingest(event), Ok(Ingest::Durable { sequence: 1 }));
    assert_eq!(ledger.ingest(event), Ok(Ingest::Duplicate { sequence: 1 }));
    assert_eq!(fixture.inspection().source_gaps(source), Ok(2));
    assert_eq!(
        ledger.ingest(Observation {
            digest: Bytes([2; 32]),
            ..event
        }),
        Err(Error::Conflict)
    );
    assert_eq!(
        ledger.ingest(Observation {
            event: id(1202),
            ..event
        }),
        Err(Error::Conflict)
    );
    assert_eq!(
        ledger.ingest(Observation {
            source: id(1002),
            ..event
        }),
        Ok(Ingest::Durable { sequence: 2 })
    );
    let late = Observation {
        event: id(1203),
        sequence: 1,
        ..event
    };
    assert_eq!(ledger.ingest(late), Ok(Ingest::Durable { sequence: 3 }));
    assert_eq!(fixture.inspection().source_gaps(source), Ok(1));
    assert_eq!(
        fixture.inspection().grant(b.grant),
        Ok(Some(State::Created))
    );
    assert_eq!(
        fixture.inspection().audit(b.grant),
        Ok(vec![(AuditKind::Transition, State::Created)])
    );
    drop(ledger);
    let mut reopened = fixture.reopen();
    assert_eq!(
        reopened.ingest(event),
        Ok(Ingest::Duplicate { sequence: 1 })
    );
    assert_eq!(
        reopened.ingest(Observation {
            event: id(1204),
            sequence: 2,
            ..event
        }),
        Ok(Ingest::Durable { sequence: 4 })
    );
    assert_eq!(fixture.inspection().source_gaps(source), Ok(0));
    assert_eq!(
        fixture.inspection().grant(b.grant),
        Ok(Some(State::Invalidated))
    );
    println!(
        "EVIDENCE AT21 ingest_sequences=1,2,3,4 source_gap=2,1,0 conflicting_source_keys=rejected"
    );
}

fn race(client: &Client, b: Binding, action: Action, count: usize) -> Vec<Result<Outcome>> {
    let barrier = Arc::new(Barrier::new(count));
    thread::scope(|scope| {
        let mut threads = Vec::new();
        for _ in 0..count {
            let barrier = barrier.clone();
            threads.push(scope.spawn(move || {
                let operation = mutate(b, action);
                barrier.wait();
                client.submit(operation)?.recv_timeout(TIMEOUT).unwrap()
            }));
        }
        threads
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect()
    })
}

#[test]
fn at22_at45_100_competing_reservations_and_consumptions() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let b = pending(&mut ledger, TARGET);
    let n = nonce(TARGET);
    let writer = Writer::start(ledger).unwrap();
    let client = writer.client();
    let reserves = race(
        &client,
        b,
        Action::Reserve {
            nonce: n,
            ttl_ms: 60_000,
        },
        100,
    );
    assert_eq!(
        reserves
            .iter()
            .filter(|r| matches!(
                r,
                Ok(Outcome::Durable(Receipt {
                    state: State::Reserved,
                    ..
                }))
            ))
            .count(),
        1
    );
    assert_eq!(
        reserves
            .iter()
            .filter(|r| **r == Err(Error::Transition))
            .count(),
        99
    );
    let consumes = race(&client, b, Action::Consume { nonce: n }, 100);
    let sink = fixture.path().join("synthetic-sink");
    for result in &consumes {
        if matches!(
            result,
            Ok(Outcome::Durable(Receipt {
                state: State::Consumed,
                ..
            }))
        ) {
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&sink)
                .unwrap()
                .sync_all()
                .unwrap();
        }
    }
    assert!(sink.is_file());
    assert_eq!(
        consumes
            .iter()
            .filter(|r| **r == Err(Error::Replay))
            .count(),
        99
    );
    drop(client);
    writer.join().unwrap();
    let evidence = inspect_child(fixture.path());
    assert!(evidence.contains("state=Consumed transitions=4 consumed=1 nonce=1 execution=unknown"));
    println!("EVIDENCE AT22/45 reserve=1/100 consume=1/100 synthetic_sink=1 {evidence}");
}

#[test]
fn at22_nonce_is_globally_unique_and_input_retries_only_reconcile() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let one = pending(&mut ledger, TARGET);
    let two = pending(&mut ledger, TARGET + 1);
    let n = nonce(TARGET);
    let operation = mutate(
        one,
        Action::Reserve {
            nonce: n,
            ttl_ms: 1000,
        },
    );
    let committed = durable(&mut ledger, operation, State::Reserved);
    assert_eq!(ledger.apply(operation), Ok(Outcome::Reconciled(committed)));
    let changed = Mutation {
        action: Action::Reserve {
            nonce: n,
            ttl_ms: 1001,
        },
        ..operation
    };
    assert_eq!(ledger.apply(changed), Err(Error::Conflict));
    assert_eq!(
        ledger.apply(mutate(two, operation.action)),
        Err(Error::Replay)
    );
    assert_eq!(
        ledger.apply(mutate(one, Action::Consume { nonce: nonce(999) })),
        Err(Error::Binding)
    );
    let wrong = Binding {
        input: Bytes([18; 32]),
        ..one
    };
    assert_eq!(
        ledger.apply(mutate(wrong, Action::Consume { nonce: n })),
        Err(Error::Binding)
    );
    for epoch in [
        Epoch {
            policy: 8,
            ..one.epoch
        },
        Epoch {
            grant: 10,
            ..one.epoch
        },
        Epoch {
            boot: id(123),
            ..one.epoch
        },
    ] {
        assert_eq!(
            ledger.apply(mutate(
                Binding { epoch, ..one },
                Action::Consume { nonce: n }
            )),
            Err(Error::Epoch)
        );
    }
    durable(
        &mut ledger,
        mutate(one, Action::Consume { nonce: n }),
        State::Consumed,
    );
    drop(ledger);
    let mut ledger = fixture.reopen();
    let new = pending(&mut ledger, TARGET + 2);
    assert_eq!(
        ledger.apply(mutate(new, operation.action)),
        Err(Error::Replay)
    );
    assert_eq!(
        fixture.inspection().grant(one.grant),
        Ok(Some(State::Consumed))
    );
    assert_eq!(
        fixture.inspection().reconcile(operation.request),
        Ok(Some(committed))
    );
}

#[test]
fn at22_rejected_request_cannot_mutate_after_conditions_change() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let b = pending(&mut ledger, TARGET);
    let n = nonce(TARGET);
    let attempt = mutate(b, Action::Consume { nonce: n });
    assert_eq!(ledger.apply(attempt), Err(Error::Transition));
    reserve(&mut ledger, b, n);
    assert_eq!(ledger.apply(attempt), Err(Error::Replay));
    assert_eq!(fixture.inspection().reconcile(attempt.request), Ok(None));
    assert_eq!(
        fixture.inspection().grant(b.grant),
        Ok(Some(State::Reserved))
    );
    durable(
        &mut ledger,
        mutate(b, Action::Consume { nonce: n }),
        State::Consumed,
    );
}

#[test]
fn at29_observation_flood_cannot_occupy_control_queue() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let b = pending(&mut ledger, TARGET);
    let (held, wait_held) = sync_channel(1);
    let (release, wait_release) = sync_channel(1);
    let mut first = true;
    ledger.set_hook(Box::new(move |stage, _| {
        if first && stage == Stage::BeforeCommit {
            first = false;
            held.send(()).unwrap();
            wait_release.recv_timeout(TIMEOUT).unwrap();
        }
        Ok(())
    }));
    let writer = Writer::start(ledger).unwrap();
    let client = writer.client();
    let reservation = client
        .submit(mutate(
            b,
            Action::Reserve {
                nonce: nonce(TARGET),
                ttl_ms: 60_000,
            },
        ))
        .unwrap();
    wait_held.recv_timeout(TIMEOUT).unwrap();
    assert!(reservation.try_recv().is_err());
    let mut samples = Vec::new();
    for index in 1..=64 {
        samples.push(
            client
                .submit_observation(Observation {
                    source: id(900),
                    event: id(index),
                    sequence: index as i64,
                    digest: Bytes([1; 32]),
                })
                .unwrap(),
        );
    }
    assert_eq!(
        client
            .submit_observation(Observation {
                source: id(900),
                event: id(65),
                sequence: 65,
                digest: Bytes([1; 32]),
            })
            .err(),
        Some(Error::QueueFull)
    );
    let cancel = client.submit(mutate(b, Action::Revoke)).unwrap();
    let mut extra_controls = Vec::new();
    for _ in 0..127 {
        extra_controls.push(client.submit(mutate(b, Action::Revoke)).unwrap());
    }
    assert_eq!(
        client.submit(mutate(b, Action::Revoke)).err(),
        Some(Error::QueueFull)
    );
    release.send(()).unwrap();
    assert!(matches!(
        reservation.recv_timeout(TIMEOUT).unwrap(),
        Ok(Outcome::Durable(_))
    ));
    assert!(matches!(
        cancel.recv_timeout(TIMEOUT).unwrap(),
        Ok(Outcome::Durable(Receipt {
            state: State::Revoked,
            ..
        }))
    ));
    for response in extra_controls {
        assert_eq!(
            response.recv_timeout(TIMEOUT).unwrap(),
            Err(Error::Transition)
        );
    }
    for response in samples {
        assert!(matches!(
            response.recv_timeout(TIMEOUT).unwrap(),
            Ok(Ingest::Durable { .. })
        ));
    }
    drop(client);
    writer.join().unwrap();
    assert_eq!(
        fixture.inspection().grant(b.grant),
        Ok(Some(State::Revoked))
    );
    println!(
        "EVIDENCE AT29 control_capacity=128 observation_capacity=64 overflow=rejected ack_before_commit=none"
    );
}

#[test]
fn at29_full_and_audit_write_failure_roll_back_and_fence() {
    for audit_failure in [false, true] {
        let fixture = Fixture::new();
        let mut ledger = fixture.create();
        let b = pending(&mut ledger, TARGET);
        let n = nonce(TARGET);
        reserve(&mut ledger, b, n);
        if audit_failure {
            ledger
                .db()
                .execute_batch(
                    "CREATE TEMP TRIGGER break_audit BEFORE INSERT ON main.audit
                 BEGIN SELECT RAISE(ABORT,'synthetic fixture audit failure'); END;",
                )
                .unwrap();
        } else {
            let pages: i64 = ledger
                .db()
                .query_row("PRAGMA page_count", [], |r| r.get(0))
                .unwrap();
            ledger
                .db()
                .pragma_update(None, "max_page_count", pages)
                .unwrap();
            ledger.set_hook(Box::new(|stage, tx| {
                if stage == Stage::BeforeAudit {
                    tx.execute_batch(
                        "CREATE TABLE fault_probe(value BLOB);
                         INSERT INTO fault_probe VALUES(zeroblob(1048576));",
                    )?;
                }
                Ok(())
            }));
        }
        let consume = mutate(b, Action::Consume { nonce: n });
        let expected = if audit_failure { 1811 } else { 13 };
        assert_eq!(ledger.apply(consume), Err(Error::Sqlite(expected)));
        assert_eq!(ledger.apply(consume), Err(Error::Fenced));
        let evidence = inspect_child(fixture.path());
        assert!(
            evidence.contains("state=Reserved transitions=3 consumed=0 nonce=1 execution=none")
        );
        assert_eq!(fixture.inspection().reconcile(consume.request), Ok(None));
        println!("EVIDENCE AT29/45 sqlite_code={expected} ack=none {evidence}");
    }
}

#[test]
fn at36_wall_rollback_monotonic_expiry_and_epoch_restart() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let clock = FakeClock::at(1000, 100);
    ledger.set_clock(Box::new(clock.clone()));
    let b = pending(&mut ledger, TARGET);
    let n = nonce(TARGET);
    durable(
        &mut ledger,
        mutate(
            b,
            Action::Reserve {
                nonce: n,
                ttl_ms: 50,
            },
        ),
        State::Reserved,
    );
    clock.set(900, 150);
    durable(
        &mut ledger,
        mutate(b, Action::Consume { nonce: n }),
        State::Expired,
    );
    let fresh = pending(&mut ledger, TARGET + 1);
    reserve(&mut ledger, fresh, nonce(TARGET + 1));
    let old_epoch = ledger.epoch();
    drop(ledger);
    let mut ledger = fixture.reopen();
    assert_ne!(ledger.epoch().boot, old_epoch.boot);
    assert_eq!(
        ledger.apply(mutate(
            fresh,
            Action::Consume {
                nonce: nonce(TARGET + 1)
            }
        )),
        Err(Error::Epoch)
    );
    assert_eq!(
        fixture.inspection().grant(b.grant),
        Ok(Some(State::Expired))
    );
    assert_eq!(
        fixture.inspection().grant(fresh.grant),
        Ok(Some(State::Invalidated))
    );
    println!(
        "EVIDENCE AT36 rollback=900<1000 mono_boundary=150 expired=1 restart_epoch_changed=1 invalidated=1"
    );
}

#[test]
fn at36_forward_clock_expiry_cannot_be_undone_and_missing_evidence_never_reserves() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let clock = FakeClock::at(1000, 100);
    ledger.set_clock(Box::new(clock.clone()));
    let b = pending(&mut ledger, TARGET);
    let n = nonce(TARGET);
    durable(
        &mut ledger,
        mutate(
            b,
            Action::Reserve {
                nonce: n,
                ttl_ms: 50,
            },
        ),
        State::Reserved,
    );
    clock.set(1100, 110);
    let consume = mutate(b, Action::Consume { nonce: n });
    let receipt = durable(&mut ledger, consume, State::Expired);
    clock.set(900, 120);
    assert_eq!(ledger.apply(consume), Ok(Outcome::Reconciled(receipt)));
    assert_eq!(
        ledger.apply(mutate(b, consume.action)),
        Err(Error::Transition)
    );
    let denied = binding(ledger.epoch(), TARGET + 1);
    durable(
        &mut ledger,
        mutate(denied, Action::Create { eligible: false }),
        State::Created,
    );
    durable(&mut ledger, mutate(denied, Action::Pend), State::Pending);
    assert_eq!(
        ledger.apply(mutate(
            denied,
            Action::Reserve {
                nonce: nonce(1),
                ttl_ms: 50
            }
        )),
        Err(Error::Ineligible)
    );
}

#[test]
fn at45_revoke_commit_order_and_synthetic_worker_failure() {
    for first_is_consume in [false, true] {
        let fixture = Fixture::new();
        let mut ledger = fixture.create();
        let b = pending(&mut ledger, TARGET);
        let n = nonce(TARGET);
        reserve(&mut ledger, b, n);
        let writer = Writer::start(ledger).unwrap();
        let client = writer.client();
        let consume = mutate(b, Action::Consume { nonce: n });
        let revoke = mutate(b, Action::Revoke);
        // Both requests enter the same queue before either caller reads an ACK.
        let (first, second) = if first_is_consume {
            (consume, revoke)
        } else {
            (revoke, consume)
        };
        let response_first = client.submit(first).unwrap();
        let response_second = client.submit(second).unwrap();
        assert!(matches!(
            response_first.recv_timeout(TIMEOUT).unwrap(),
            Ok(Outcome::Durable(_))
        ));
        let second = response_second.recv_timeout(TIMEOUT).unwrap();
        if first_is_consume {
            assert!(matches!(
                second,
                Ok(Outcome::Durable(Receipt {
                    kind: AuditKind::CancelRequested,
                    ..
                }))
            ));
            let failed = child_output("worker-failed", fixture.path());
            assert!(!failed.status.success());
            assert!(fixture.path().join("worker-entered").is_file());
            assert!(matches!(
                client
                    .submit(mutate(b, Action::WorkerFailed))
                    .unwrap()
                    .recv_timeout(TIMEOUT)
                    .unwrap(),
                Ok(Outcome::Durable(Receipt {
                    kind: AuditKind::WorkerFailed,
                    state: State::Consumed,
                    ..
                }))
            ));
        } else {
            assert_eq!(second, Err(Error::Transition));
        }
        drop(client);
        writer.join().unwrap();
        let evidence = inspect_child(fixture.path());
        if first_is_consume {
            assert!(evidence.contains(
                "state=Consumed transitions=4 consumed=1 nonce=1 execution=failed cancel=1"
            ));
        } else {
            assert!(evidence.contains(
                "state=Revoked transitions=4 consumed=0 nonce=1 execution=none cancel=0"
            ));
        }
        println!("EVIDENCE AT45 consume_first={first_is_consume} {evidence}");
    }
}

#[test]
fn at45_unknown_commit_only_reconciles_and_writer_is_fenced() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let b = pending(&mut ledger, TARGET);
    let n = nonce(TARGET);
    reserve(&mut ledger, b, n);
    ledger.set_hook(Box::new(|stage, _| {
        if stage == Stage::AfterCommit {
            Err(Error::Storage)
        } else {
            Ok(())
        }
    }));
    let command = mutate(b, Action::Consume { nonce: n });
    assert_eq!(ledger.apply(command), Err(Error::UnknownCommit));
    assert_eq!(ledger.apply(command), Err(Error::Fenced));
    let evidence = inspect_child(fixture.path());
    assert!(evidence.contains("state=Consumed transitions=4 consumed=1 nonce=1 execution=unknown"));
    let receipt = fixture
        .inspection()
        .reconcile(command.request)
        .unwrap()
        .unwrap();
    drop(ledger);
    let mut reopened = fixture.reopen();
    assert_eq!(reopened.apply(command), Ok(Outcome::Reconciled(receipt)));
    assert_eq!(reopened.apply(mutate(b, command.action)), Err(Error::Epoch));
    println!(
        "EVIDENCE AT45 unknown_commit=no_ACK fenced=true reconciled_sequence={} {evidence}",
        receipt.sequence
    );
}

#[test]
fn at45_actual_commit_error_returns_no_receipt_and_preserves_reserved() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let b = pending(&mut ledger, TARGET);
    let n = nonce(TARGET);
    reserve(&mut ledger, b, n);
    ledger
        .db()
        .execute_batch(
            "CREATE TEMP TABLE deferred_parent(id INTEGER PRIMARY KEY);
         CREATE TEMP TABLE deferred_fault(id INTEGER REFERENCES deferred_parent(id)
            DEFERRABLE INITIALLY DEFERRED);",
        )
        .unwrap();
    ledger.set_hook(Box::new(|stage, tx| {
        if stage == Stage::BeforeCommit {
            tx.execute("INSERT INTO deferred_fault VALUES(1)", [])?;
        }
        Ok(())
    }));
    let consume = mutate(b, Action::Consume { nonce: n });
    assert_eq!(ledger.apply(consume), Err(Error::UnknownCommit));
    assert_eq!(ledger.apply(consume), Err(Error::Fenced));
    assert_eq!(fixture.inspection().reconcile(consume.request), Ok(None));
    let evidence = inspect_child(fixture.path());
    assert!(evidence.contains("state=Reserved transitions=3 consumed=0 nonce=1 execution=none"));
    drop(ledger);
    let mut reopened = fixture.reopen();
    assert_eq!(reopened.apply(consume), Err(Error::Epoch));
    assert_eq!(
        fixture.inspection().grant(b.grant),
        Ok(Some(State::Invalidated))
    );
    println!("EVIDENCE AT45 commit_constraint=deferred_fk ack=none receipt=none {evidence}");
}

#[test]
fn at45_revoke_and_consume_race_from_separate_callers() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let b = pending(&mut ledger, TARGET);
    let n = nonce(TARGET);
    reserve(&mut ledger, b, n);
    let writer = Writer::start(ledger).unwrap();
    let client = writer.client();
    let barrier = Barrier::new(2);
    let (consume, revoke) = thread::scope(|scope| {
        let consume = scope.spawn(|| {
            barrier.wait();
            client
                .submit(mutate(b, Action::Consume { nonce: n }))
                .unwrap()
                .recv_timeout(TIMEOUT)
                .unwrap()
        });
        let revoke = scope.spawn(|| {
            barrier.wait();
            client
                .submit(mutate(b, Action::Revoke))
                .unwrap()
                .recv_timeout(TIMEOUT)
                .unwrap()
        });
        (consume.join().unwrap(), revoke.join().unwrap().unwrap())
    });
    drop(client);
    writer.join().unwrap();
    let evidence = inspect_child(fixture.path());
    match (consume, revoke) {
        (
            Err(Error::Transition),
            Outcome::Durable(Receipt {
                state: State::Revoked,
                ..
            }),
        ) => assert!(
            evidence
                .contains("state=Revoked transitions=4 consumed=0 nonce=1 execution=none cancel=0")
        ),
        (Ok(Outcome::Durable(consumed)), Outcome::Durable(cancel))
            if consumed.state == State::Consumed && cancel.kind == AuditKind::CancelRequested =>
        {
            assert!(consumed.sequence < cancel.sequence);
            assert!(evidence.contains(
                "state=Consumed transitions=4 consumed=1 nonce=1 execution=unknown cancel=1"
            ));
        }
        _ => panic!("unexpected race results: {consume:?} {revoke:?}"),
    }
    println!("EVIDENCE AT45 simultaneous_revoke_consume {evidence}");
}

#[test]
fn at36_expire_action_handles_preparation_and_reserved_deadlines() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let clock = FakeClock::at(1000, 100);
    ledger.set_clock(Box::new(clock.clone()));
    let created = create(&mut ledger, TARGET);
    let pending_grant = pending(&mut ledger, TARGET + 1);
    let reserved = pending(&mut ledger, TARGET + 2);
    durable(&mut ledger, mutate(created, Action::Expire), State::Expired);
    durable(
        &mut ledger,
        mutate(pending_grant, Action::Expire),
        State::Expired,
    );
    durable(
        &mut ledger,
        mutate(
            reserved,
            Action::Reserve {
                nonce: nonce(TARGET + 2),
                ttl_ms: 50,
            },
        ),
        State::Reserved,
    );
    assert_eq!(
        ledger.apply(mutate(reserved, Action::Expire)),
        Err(Error::Transition)
    );
    clock.set(900, 151);
    durable(
        &mut ledger,
        mutate(reserved, Action::Expire),
        State::Expired,
    );
    assert_eq!(
        fixture.inspection().grant(reserved.grant),
        Ok(Some(State::Expired))
    );
}

#[test]
fn at45_database_rejects_all_49_illegal_state_pairs() {
    let mut allowed = 0;
    for start in State::ALL {
        let fixture = Fixture::new();
        let mut ledger = fixture.create();
        let clock = FakeClock::at(1000, 100);
        ledger.set_clock(Box::new(clock.clone()));
        let b = create(&mut ledger, TARGET);
        let n = nonce(TARGET);
        match start {
            State::Created => {}
            State::Pending | State::Reserved | State::Consumed | State::Expired => {
                durable(&mut ledger, mutate(b, Action::Pend), State::Pending);
                if start != State::Pending {
                    reserve(&mut ledger, b, n);
                }
                if start == State::Expired {
                    clock.set(61_000, 100);
                }
                if matches!(start, State::Consumed | State::Expired) {
                    durable(&mut ledger, mutate(b, Action::Consume { nonce: n }), start);
                }
            }
            State::Revoked => {
                durable(&mut ledger, mutate(b, Action::Revoke), start);
            }
            State::Invalidated => {
                durable(&mut ledger, mutate(b, Action::Invalidate), start);
            }
            State::Failed => {
                durable(&mut ledger, mutate(b, Action::FailPreparation), start);
            }
        }
        for end in State::ALL {
            let legal = match start {
                State::Created => matches!(
                    end,
                    State::Pending
                        | State::Revoked
                        | State::Expired
                        | State::Invalidated
                        | State::Failed
                ),
                State::Pending => matches!(
                    end,
                    State::Reserved
                        | State::Revoked
                        | State::Expired
                        | State::Invalidated
                        | State::Failed
                ),
                State::Reserved => matches!(
                    end,
                    State::Consumed
                        | State::Revoked
                        | State::Expired
                        | State::Invalidated
                        | State::Failed
                ),
                _ => false,
            };
            let tx = ledger.db().unchecked_transaction().unwrap();
            let actual = tx.execute(
                "UPDATE grants SET state=?,wall_deadline=100,mono_deadline=100 WHERE id=?",
                (end.name(), b.grant),
            );
            assert_eq!(actual.is_ok(), legal, "{start:?} -> {end:?}");
            assert_eq!(start.permits(end), legal);
            allowed += usize::from(legal);
            tx.rollback().unwrap();
        }
        assert_eq!(fixture.inspection().grant(b.grant), Ok(Some(start)));
    }
    assert_eq!(allowed, 15);
    println!("EVIDENCE AT45 database_pairs=64 legal=15 rejected=49");
}

#[test]
fn migration_and_archive_preserve_history_and_never_restore_active_grants() {
    let legacy = Fixture::new();
    File::create_new(legacy.path().join(db::DB)).unwrap();
    let raw = db::connection(
        &legacy.path().join(db::DB),
        OpenFlags::SQLITE_OPEN_READ_WRITE,
    )
    .unwrap();
    raw.pragma_update(None, "application_id", db::APPLICATION_ID)
        .unwrap();
    raw.pragma_update(None, "user_version", 1).unwrap();
    raw.execute_batch(db::V1).unwrap();
    let original = Epoch {
        boot: id(6),
        policy: POLICY,
        grant: GRANT_EPOCH,
    };
    let old = binding(original, TARGET);
    raw.execute(
        "INSERT INTO meta VALUES(1,?,?,?,0)",
        (original.boot, original.policy, original.grant),
    )
    .unwrap();
    raw.execute(
        "INSERT INTO decisions VALUES(?,?,?,?,?,1)",
        (
            old.decision,
            old.input,
            original.boot,
            original.policy,
            original.grant,
        ),
    )
    .unwrap();
    raw.execute(
        "INSERT INTO grants(id,decision,state) VALUES(?,?,'created')",
        (old.grant, old.decision),
    )
    .unwrap();
    raw.execute(
        "INSERT INTO audit(grant_id,kind,state) VALUES(?,'transition','created')",
        [old.grant],
    )
    .unwrap();
    drop(raw);
    let mut ledger = legacy.reopen();
    assert_eq!(db::version(ledger.db()), Ok(2));
    assert_eq!(
        legacy.inspection().grant(old.grant),
        Ok(Some(State::Invalidated))
    );
    assert_eq!(legacy.inspection().audit(old.grant).unwrap().len(), 2);
    let b = pending(&mut ledger, TARGET + 1);
    let n = nonce(TARGET + 1);
    reserve(&mut ledger, b, n);
    let snapshot = Fixture::new();
    ledger.backup_to(snapshot.path()).unwrap();
    durable(
        &mut ledger,
        mutate(b, Action::Consume { nonce: n }),
        State::Consumed,
    );
    let archive = Archive::open(snapshot.path()).unwrap();
    assert_eq!(archive.history().grant(b.grant), Ok(Some(State::Reserved)));
    assert_eq!(
        legacy.inspection().grant(b.grant),
        Ok(Some(State::Consumed))
    );
    assert!(Ledger::reopen(snapshot.path(), POLICY, GRANT_EPOCH).is_err());
    assert_eq!(
        Ledger::create(snapshot.path(), POLICY, GRANT_EPOCH).err(),
        Some(Error::Conflict)
    );
    let copied = Fixture::new();
    fs::copy(
        snapshot.path().join("history.sqlite3"),
        copied.path().join(db::DB),
    )
    .unwrap();
    assert_eq!(
        Ledger::reopen(copied.path(), POLICY, GRANT_EPOCH).err(),
        Some(Error::Archived)
    );
    assert_eq!(Inspection::open(copied.path()).err(), Some(Error::Archived));
    println!(
        "EVIDENCE AT36/45 schema=1->2 old_grant=invalidated archive=read_only archived_reactivation=denied live=consumed"
    );
}

#[test]
fn integrity_schema_and_version_fail_closed_without_reinitializing() {
    for case in ["version", "state", "schema"] {
        let fixture = Fixture::new();
        let mut ledger = fixture.create();
        let b = pending(&mut ledger, TARGET);
        match case {
            "version" => ledger.db().pragma_update(None, "user_version", 99).unwrap(),
            "state" => {
                ledger
                    .db()
                    .execute("UPDATE grants SET state='revoked' WHERE id=?", [b.grant])
                    .unwrap();
            }
            _ => ledger
                .db()
                .execute_batch("DROP TRIGGER grant_transition;")
                .unwrap(),
        }
        drop(ledger);
        let before = fs::read(fixture.path().join(db::DB)).unwrap();
        let expected = if case == "version" {
            Error::Version
        } else {
            Error::Corrupt
        };
        assert_eq!(
            Ledger::reopen(fixture.path(), POLICY, GRANT_EPOCH).err(),
            Some(expected)
        );
        assert_eq!(fs::read(fixture.path().join(db::DB)).unwrap(), before);
        assert_eq!(
            Ledger::create(fixture.path(), POLICY, GRANT_EPOCH).err(),
            Some(Error::Conflict)
        );
    }
    let fixture = Fixture::new();
    fs::write(fixture.path().join(db::DB), b"synthetic damaged database").unwrap();
    assert_eq!(
        Ledger::reopen(fixture.path(), POLICY, GRANT_EPOCH).err(),
        Some(Error::Corrupt)
    );
    assert_eq!(
        fs::read(fixture.path().join(db::DB)).unwrap(),
        b"synthetic damaged database"
    );
}

fn child_process(mode: &str, path: &Path) -> Child {
    use std::os::windows::process::CommandExt;
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "tests::child_fixture",
            "--ignored",
            "--nocapture",
        ])
        .env("AG_I05_TEST_CHILD", mode)
        .current_dir(path)
        .creation_flags(0x0800_0000)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn get(&mut self) -> &mut Child {
        self.0.as_mut().unwrap()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn child_output(mode: &str, path: &Path) -> Output {
    let mut child = ChildGuard(Some(child_process(mode, path)));
    let start = Instant::now();
    while child.get().try_wait().unwrap().is_none() {
        assert!(start.elapsed() < TIMEOUT, "child {mode} timed out");
        thread::sleep(Duration::from_millis(10));
    }
    child.0.take().unwrap().wait_with_output().unwrap()
}

fn inspect_child(path: &Path) -> String {
    let output = child_output("inspect", path);
    assert!(output.status.success(), "{:?}", output);
    let text = String::from_utf8(output.stdout).unwrap();
    text.lines()
        .find_map(|line| {
            line.split_once("EVIDENCE-CHILD ")
                .map(|(_, evidence)| evidence.trim().to_owned())
        })
        .unwrap()
}

fn hold_child(path: &Path, mode: &str) -> ChildGuard {
    let mut child = ChildGuard(Some(child_process(mode, path)));
    let start = Instant::now();
    while !path.join("fault-ready").is_file() {
        assert!(
            child.get().try_wait().unwrap().is_none(),
            "child {mode} exited early"
        );
        assert!(
            start.elapsed() < TIMEOUT,
            "child {mode} failed to reach failpoint"
        );
        thread::sleep(Duration::from_millis(10));
    }
    child
}

fn stop_child(child: &mut ChildGuard) {
    assert!(child.get().try_wait().unwrap().is_none());
    child.get().kill().unwrap();
    assert!(!child.get().wait().unwrap().success());
}

fn stop_here() -> ! {
    File::create_new("fault-ready").unwrap().sync_all().unwrap();
    loop {
        thread::park();
    }
}

#[test]
fn at29_at45_independent_sqlite_busy_and_writer_lock_are_distinct() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let b = pending(&mut ledger, TARGET);
    let n = nonce(TARGET);
    reserve(&mut ledger, b, n);
    let app = child_output("writer-contender", fixture.path());
    assert!(app.status.success(), "{app:?}");
    let mut raw = hold_child(fixture.path(), "sqlite-busy");
    let command = mutate(b, Action::Consume { nonce: n });
    let start = Instant::now();
    assert_eq!(ledger.apply(command), Err(Error::Busy));
    assert!(start.elapsed() < Duration::from_secs(5));
    stop_child(&mut raw);
    let evidence = inspect_child(fixture.path());
    assert!(evidence.contains("state=Reserved transitions=3 consumed=0 nonce=1 execution=none"));
    assert_eq!(fixture.inspection().reconcile(command.request), Ok(None));
    assert_eq!(ledger.apply(command), Err(Error::Fenced));
    drop(ledger);
    let mut reopened = fixture.reopen();
    assert_eq!(reopened.apply(command), Err(Error::Epoch));
    assert_eq!(
        fixture.inspection().grant(b.grant),
        Ok(Some(State::Invalidated))
    );
    println!(
        "EVIDENCE AT29/45 cross_process_writer=busy sqlite_busy=5 no_partial_commit retry=fenced_and_invalidated {evidence}"
    );
}

#[test]
fn at45_process_crashes_at_reserve_consume_audit_response_and_launch() {
    let cases = [
        ("reserve-before-audit", State::Pending, 2, false),
        ("reserve-before-commit", State::Pending, 2, false),
        ("consume-before-audit", State::Reserved, 3, false),
        ("consume-before-commit", State::Reserved, 3, false),
        ("response-before-send", State::Consumed, 4, false),
        ("launch-before-marker", State::Consumed, 4, false),
        ("launch-after-marker", State::Consumed, 4, true),
    ];
    for (mode, expected, transitions, launched) in cases {
        let fixture = Fixture::new();
        let mut child = hold_child(fixture.path(), mode);
        let previous = fixture.inspection().epoch().unwrap();
        let b = binding(previous, TARGET);
        stop_child(&mut child);
        let wal_bytes = fs::metadata(fixture.path().join("ledger.sqlite3-wal"))
            .unwrap()
            .len();
        assert!(wal_bytes > 32);
        let recorded = inspect_child(fixture.path());
        assert!(recorded.contains(&format!("state={expected:?} transitions={transitions}")));
        assert_eq!(fixture.path().join("synthetic-sink").is_file(), launched);
        let mut reopened = fixture.reopen();
        assert_ne!(reopened.epoch().boot, previous.boot);
        let action = if mode.starts_with("reserve") {
            Action::Reserve {
                nonce: nonce(TARGET),
                ttl_ms: 60_000,
            }
        } else {
            Action::Consume {
                nonce: nonce(TARGET),
            }
        };
        let retry = reopened.apply(Mutation {
            request: id(6000),
            binding: b,
            action,
        });
        if expected == State::Consumed {
            assert!(matches!(
                retry,
                Ok(Outcome::Reconciled(Receipt {
                    state: State::Consumed,
                    ..
                }))
            ));
        } else {
            assert_eq!(retry, Err(Error::Epoch));
        }
        let state = if expected == State::Consumed {
            State::Consumed
        } else {
            State::Invalidated
        };
        assert_eq!(fixture.inspection().grant(b.grant), Ok(Some(state)));
        assert_eq!(fixture.path().join("synthetic-sink").is_file(), launched);
        drop(reopened);
        let post_restart = inspect_child(fixture.path());
        assert!(post_restart.contains(&format!("state={state:?}")));
        println!(
            "EVIDENCE AT45 fault={mode} wal_bytes={wal_bytes} sink={launched} before=[{recorded}] after=[{post_restart}]"
        );
    }
}

#[test]
#[ignore = "child-process driver; invoked by the AT suite"]
fn child_fixture() {
    let Ok(mode) = std::env::var("AG_I05_TEST_CHILD") else {
        return;
    };
    let path = std::env::current_dir().unwrap();
    let path = db::fixture_dir(&path).unwrap();
    match mode.as_str() {
        "inspect" => {
            Inspection::open(&path).unwrap().verify().unwrap();
            let db = db::connection(&path.join(db::DB), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            db.execute_batch("PRAGMA query_only=ON;").unwrap();
            let grant = id(TARGET);
            let result: (State, i64, i64, i64, String, i64) = db.query_row(
                "SELECT g.state,
                   (SELECT count(*) FROM audit a WHERE a.grant_id=g.id AND a.kind='transition'),
                   (SELECT count(*) FROM audit a WHERE a.grant_id=g.id AND a.kind='transition' AND a.state='consumed'),
                   (SELECT count(*) FROM nonces n WHERE n.grant_id=g.id),
                   coalesce((SELECT state FROM execution e WHERE e.grant_id=g.id),'none'),
                   coalesce((SELECT cancel_requested FROM execution e WHERE e.grant_id=g.id),0)
                 FROM grants g WHERE g.id=?",
                [grant], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
            ).unwrap();
            println!(
                "EVIDENCE-CHILD state={:?} transitions={} consumed={} nonce={} execution={} cancel={}",
                result.0, result.1, result.2, result.3, result.4, result.5
            );
        }
        "writer-contender" => assert_eq!(
            Ledger::reopen(&path, POLICY, GRANT_EPOCH).err(),
            Some(Error::Busy)
        ),
        "sqlite-busy" => {
            let db = db::connection(&path.join(db::DB), OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
            db.execute_batch("BEGIN IMMEDIATE").unwrap();
            stop_here();
        }
        "worker-failed" => {
            File::create_new("worker-entered")
                .unwrap()
                .sync_all()
                .unwrap();
            std::process::exit(17);
        }
        "reserve-before-audit"
        | "reserve-before-commit"
        | "consume-before-audit"
        | "consume-before-commit"
        | "response-before-send"
        | "launch-before-marker"
        | "launch-after-marker" => {
            let mut ledger = Ledger::create(&path, POLICY, GRANT_EPOCH).unwrap();
            let b = pending(&mut ledger, TARGET);
            let n = nonce(TARGET);
            let action = if mode.starts_with("reserve") {
                Action::Reserve {
                    nonce: n,
                    ttl_ms: 60_000,
                }
            } else {
                reserve(&mut ledger, b, n);
                Action::Consume { nonce: n }
            };
            let stage = match mode.as_str() {
                "reserve-before-audit" | "consume-before-audit" => Some(Stage::BeforeAudit),
                "reserve-before-commit" | "consume-before-commit" => Some(Stage::BeforeCommit),
                "response-before-send" => Some(Stage::AfterCommit),
                _ => None,
            };
            if let Some(stage) = stage {
                ledger.set_hook(Box::new(move |reached, _| {
                    if reached == stage {
                        stop_here();
                    }
                    Ok(())
                }));
            }
            durable(
                &mut ledger,
                Mutation {
                    request: id(6000),
                    binding: b,
                    action,
                },
                State::Consumed,
            );
            if mode == "launch-after-marker" {
                File::create_new("synthetic-sink")
                    .unwrap()
                    .sync_all()
                    .unwrap();
            }
            stop_here();
        }
        _ => panic!("unknown fixture child"),
    }
}
