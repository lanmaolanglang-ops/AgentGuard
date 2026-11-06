use super::*;
use rusqlite::{Connection, Error as SqlError, ErrorCode, params, types::Value};
use sha2::{Digest as _, Sha256};

fn open_fixture(fixture: &Fixture, flags: OpenFlags) -> Connection {
    db::connection(&fixture.path().join(db::DB), flags).unwrap()
}

fn facts(fixture: &Fixture) -> String {
    let reader = open_fixture(fixture, OpenFlags::SQLITE_OPEN_READ_ONLY);
    reader.execute_batch("PRAGMA query_only=ON;").unwrap();
    db::integrity(&reader).unwrap();
    let mut hash = Sha256::new();
    for table in [
        "meta",
        "decisions",
        "grants",
        "nonces",
        "execution",
        "audit",
        "g0_cutover",
        "g0_heads",
        "g0_reservations",
        "g0_execution_bindings",
    ] {
        let mut query = reader
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap();
        let columns = query.column_count();
        let rows = query
            .query_map([], |row| {
                (0..columns)
                    .map(|column| row.get::<_, Value>(column))
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        hash.update(format!("{table}={rows:?}\n"));
    }
    format!("{:x}", hash.finalize())
}

fn reject(
    fixture: &Fixture,
    case: &str,
    recursive: bool,
    deferred: bool,
    attempt: impl FnOnce(&Connection) -> rusqlite::Result<()>,
) {
    let before = facts(fixture);
    let db = open_fixture(fixture, OpenFlags::SQLITE_OPEN_READ_WRITE);
    db.pragma_update(None, "recursive_triggers", recursive)
        .unwrap();
    assert_eq!(
        db.pragma_query_value(None, "recursive_triggers", |row| row.get::<_, bool>(0))
            .unwrap(),
        recursive
    );
    let tx = db.unchecked_transaction().unwrap();
    let result = attempt(&tx);
    let error = if deferred {
        result.unwrap_or_else(|e| panic!("{case} did not reach commit: {e}"));
        tx.commit().expect_err(case)
    } else {
        let error = result.expect_err(case);
        tx.rollback().unwrap();
        error
    };
    assert!(
        matches!(&error, SqlError::SqliteFailure(code,_) if code.code==ErrorCode::ConstraintViolation),
        "{case}: {error}"
    );
    if deferred {
        assert!(
            matches!(&error, SqlError::SqliteFailure(code,_) if code.extended_code==787),
            "{case}: {error}"
        );
    }
    assert_eq!(facts(fixture), before, "{case} left partial changes");
    println!(
        "EVIDENCE {case} recursive={} rejected_at={} independent_snapshot_sha256={before}",
        u8::from(recursive),
        if deferred { "commit" } else { "statement" }
    );
}

fn execute(db: &Connection, sql: &str, args: impl rusqlite::Params) -> rusqlite::Result<()> {
    db.execute(sql, args).map(|_| ())
}

pub(super) fn read_binding_counts(db: &Connection, grant: Id) -> String {
    let (reservations, executions): (i64, i64) = db
        .query_row(
            "SELECT (SELECT count(*) FROM g0_reservations WHERE grant_id=?1),
                (SELECT count(*) FROM g0_execution_bindings WHERE grant_id=?1)",
            [grant],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    format!("reservation_bindings={reservations} execution_bindings={executions}")
}

pub(super) fn stage_reservation(
    db: &Connection,
    b: Binding,
    n: Digest,
    insert_nonce: bool,
) -> rusqlite::Result<()> {
    db.execute(
        "INSERT INTO g0_reservations(grant_id,decision_id,operation_digest,boot,policy,grant_epoch,
            nonce_hash,wall_deadline,mono_deadline) VALUES(?,?,?,?,?,?,?,1100,200)",
        params![
            b.grant,
            b.decision,
            b.input,
            b.epoch.boot,
            b.epoch.policy,
            b.epoch.grant,
            n
        ],
    )?;
    if insert_nonce {
        db.execute("INSERT INTO nonces VALUES(?,?)", (n, b.grant))?;
    }
    Ok(())
}

pub(super) fn stage_execution(db: &Connection, b: Binding) -> rusqlite::Result<()> {
    db.execute(
        "INSERT INTO g0_execution_bindings(grant_id,operation_digest) VALUES(?,?)",
        (b.grant, b.input),
    )?;
    execute(db, "INSERT INTO execution VALUES(?,'unknown',0)", [b.grant])
}

fn raw_reserve(db: &Connection, grant: Id) -> rusqlite::Result<()> {
    execute(
        db,
        "UPDATE grants SET state='reserved',wall_deadline=1100,mono_deadline=200 WHERE id=?",
        [grant],
    )
}

#[test]
fn g0a_immutable_history_with_recursive_triggers_on_and_off() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let original = create(&mut ledger, TARGET);
    let new_id = id(900);
    let new_decision = id(901);
    for recursive in [true, false] {
        reject(&fixture, "G0a-01", recursive, false, |tx| {
            execute(
                tx,
                "UPDATE grants SET id=? WHERE id=?",
                (new_id, original.grant),
            )
        });
        reject(&fixture, "G0a-02", recursive, false, |tx| {
            execute(
                tx,
                "INSERT INTO decisions SELECT ?,input_digest,boot,policy,grant_epoch,eligible
                       FROM decisions WHERE id=?",
                (new_decision, original.decision),
            )?;
            execute(
                tx,
                "UPDATE grants SET decision=? WHERE id=?",
                (new_decision, original.grant),
            )
        });
        reject(&fixture, "G0a-03", recursive, false, |tx| {
            execute(tx, "DELETE FROM grants WHERE id=?", [original.grant])
        });
        reject(&fixture, "G0a-04", recursive, false, |tx| {
            execute(tx, "DELETE FROM decisions WHERE id=?", [original.decision])
        });
        reject(&fixture, "G0a-05", recursive, false, |tx| {
            execute(
                tx,
                "INSERT OR REPLACE INTO grants(id,decision,state) VALUES(?,?,'created')",
                (original.grant, original.decision),
            )
        });
        reject(&fixture, "G0a-06", recursive, false, |tx| {
            execute(
                tx,
                "INSERT OR REPLACE INTO decisions
                SELECT id,?,boot,policy,grant_epoch,eligible FROM decisions WHERE id=?",
                (Bytes([88; 32]), original.decision),
            )
        });
        reject(&fixture, "G0a-07", recursive, false, |tx| {
            execute(
                tx,
                "INSERT INTO decisions SELECT ?,input_digest,boot,policy,grant_epoch,eligible
                         FROM decisions WHERE id=?",
                (new_decision, original.decision),
            )?;
            execute(
                tx,
                "INSERT INTO grants(id,decision,state) VALUES(?,?,'created')
                ON CONFLICT(id) DO UPDATE SET decision=excluded.decision",
                (original.grant, new_decision),
            )
        });
        reject(&fixture, "G0a-08", recursive, false, |tx| {
            execute(tx, "DELETE FROM grants WHERE id=?", [original.grant])?;
            execute(
                tx,
                "INSERT INTO grants(id,decision,state) VALUES(?,?,'created')",
                (original.grant, original.decision),
            )
        });
        reject(&fixture, "G0a-09", recursive, false, |tx| {
            execute(tx,"INSERT OR REPLACE INTO decisions(rowid,id,input_digest,boot,policy,grant_epoch,eligible)
                SELECT rowid,?,input_digest,boot,policy,grant_epoch,eligible FROM decisions WHERE id=?",
                (new_decision,original.decision))
        });
        reject(&fixture, "G0a-09-decision-unique", recursive, false, |tx| {
            execute(
                tx,
                "INSERT OR REPLACE INTO grants(id,decision,state) VALUES(?,?,'created')",
                (new_id, original.decision),
            )
        });
    }
    let next = create(&mut ledger, TARGET + 1);
    assert_eq!(
        fixture.inspection().grant(next.grant),
        Ok(Some(State::Created))
    );
    let snapshot = facts(&fixture);
    println!("EVIDENCE G0a-10 legal_create=durable independent_snapshot_sha256={snapshot}");
}

#[test]
fn g0b_deadline_columns_are_written_once_and_are_null_safe() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    ledger.set_clock(Box::new(FakeClock::at(1000, 100)));
    let b = pending(&mut ledger, TARGET);
    reject(&fixture, "G0b-09", true, false, |tx| {
        execute(
            tx,
            "UPDATE grants SET wall_deadline=1100,mono_deadline=200 WHERE id=?",
            [b.grant],
        )
    });
    let n = nonce(TARGET);
    durable(
        &mut ledger,
        mutate(
            b,
            Action::Reserve {
                nonce: n,
                ttl_ms: 100,
            },
        ),
        State::Reserved,
    );
    let read = open_fixture(&fixture, OpenFlags::SQLITE_OPEN_READ_ONLY);
    let deadline: (i64, i64, Digest) = read
        .query_row(
            "SELECT g.wall_deadline,g.mono_deadline,r.nonce_hash FROM grants g
         JOIN g0_reservations r ON r.grant_id=g.id WHERE g.id=?",
            [b.grant],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(deadline, (1100, 200, n));
    drop(read);
    println!(
        "EVIDENCE G0b-10 initial_pair=1100,200 nonce_matches=true independent_reader=verified"
    );
    for (case, set) in [
        ("G0b-01", "wall_deadline=1200"),
        ("G0b-02", "mono_deadline=300"),
        ("G0b-03", "wall_deadline=1200,mono_deadline=300"),
        ("G0b-04", "wall_deadline=1099"),
        ("G0b-05", "wall_deadline=NULL,mono_deadline=NULL"),
        (
            "G0b-06",
            "state='revoked',wall_deadline=1200,mono_deadline=300",
        ),
        ("G0b-08", "state='reserved',wall_deadline=1200"),
    ] {
        reject(&fixture, case, true, false, |tx| {
            execute(
                tx,
                &format!("UPDATE grants SET {set} WHERE id=?"),
                [b.grant],
            )
        });
    }
    durable(
        &mut ledger,
        mutate(b, Action::Consume { nonce: n }),
        State::Consumed,
    );
    reject(&fixture, "G0b-07", true, false, |tx| {
        execute(
            tx,
            "UPDATE grants SET mono_deadline=999 WHERE id=?",
            [b.grant],
        )
    });
}

#[test]
fn g0b_wall_and_monotonic_boundaries_are_observed_from_disk() {
    for (label, wall, mono, expected) in [
        ("before", 1099, 199, State::Consumed),
        ("utc_equal", 1100, 199, State::Expired),
        ("utc_after", 1101, 199, State::Expired),
        ("mono_equal", 1099, 200, State::Expired),
        ("mono_after", 1099, 201, State::Expired),
        ("rollback_equal", 900, 200, State::Expired),
        ("rollback_after", 900, 201, State::Expired),
    ] {
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
                    ttl_ms: 100,
                },
            ),
            State::Reserved,
        );
        clock.set(wall, mono);
        durable(
            &mut ledger,
            mutate(b, Action::Consume { nonce: n }),
            expected,
        );
        let disk = fixture.inspection();
        assert_eq!(disk.grant(b.grant), Ok(Some(expected)));
        assert_eq!(
            disk.execution(b.grant).unwrap().is_some(),
            expected == State::Consumed
        );
        println!("EVIDENCE G0b-time {label} wall={wall} mono={mono} state={expected:?}");
    }
    println!("EVIDENCE G0b-11 rollback_not_extended=verified");
    println!("EVIDENCE G0b-12 monotonic_equal_and_after=expired_no_execution");
}

#[test]
fn g0c_supported_raw_writes_require_all_companions() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    ledger.set_clock(Box::new(FakeClock::at(1000, 100)));
    let b = pending(&mut ledger, TARGET);
    let n = nonce(TARGET);
    let other = pending(&mut ledger, TARGET + 1);
    let other_nonce = nonce(TARGET + 1);
    reserve(&mut ledger, other, other_nonce);
    reject(&fixture, "G0c-01", true, false, |tx| {
        stage_reservation(tx, b, n, false)?;
        raw_reserve(tx, b.grant)
    });
    reject(&fixture, "G0c-02", true, false, |tx| {
        stage_reservation(tx, b, other_nonce, false)?;
        raw_reserve(tx, b.grant)
    });
    reject(&fixture, "G0c-03", true, false, |tx| {
        stage_reservation(tx, b, other_nonce, true)
    });
    reject(&fixture, "G0c-04", true, true, |tx| {
        stage_reservation(tx, b, n, true)?;
        raw_reserve(tx, b.grant)
    });
    reject(&fixture, "G0c-08", true, false, |tx| {
        stage_reservation(tx, b, n, true)?;
        raw_reserve(tx, b.grant)?;
        execute(
            tx,
            "INSERT INTO audit(grant_id,kind,previous,state)
                 VALUES(?,'transition','pending','revoked')",
            [b.grant],
        )
    });
    reserve(&mut ledger, b, n);
    reject(&fixture, "G0c-05", true, false, |tx| {
        execute(
            tx,
            "UPDATE grants SET state='consumed' WHERE id=?",
            [b.grant],
        )
    });
    reject(&fixture, "G0c-06", true, true, |tx| {
        stage_execution(tx, b)?;
        execute(
            tx,
            "UPDATE grants SET state='consumed' WHERE id=?",
            [b.grant],
        )
    });
    reject(&fixture, "G0c-07", true, false, |tx| {
        execute(
            tx,
            "INSERT INTO g0_execution_bindings(grant_id,operation_digest) VALUES(?,?)",
            (b.grant, b.input),
        )?;
        execute(
            tx,
            "INSERT INTO execution VALUES(?,'unknown',0)",
            [other.grant],
        )
    });
    reject(&fixture, "G0c-09", true, false, |tx| {
        execute(
            tx,
            "UPDATE g0_reservations SET operation_digest=? WHERE grant_id=?",
            (Bytes([3; 32]), b.grant),
        )
    });
    reject(&fixture, "G0c-09-execution", true, false, |tx| {
        execute(
            tx,
            "INSERT INTO g0_execution_bindings(grant_id,operation_digest) VALUES(?,?)",
            (b.grant, Bytes([3; 32])),
        )
    });
    let changed = Binding {
        input: Bytes([3; 32]),
        ..b
    };
    assert_eq!(
        ledger.apply(mutate(changed, Action::Consume { nonce: n })),
        Err(Error::Binding)
    );
    durable(
        &mut ledger,
        mutate(b, Action::Consume { nonce: n }),
        State::Consumed,
    );
    reject(&fixture, "G0c-09-sealed-execution", true, false, |tx| {
        execute(
            tx,
            "UPDATE g0_execution_bindings SET operation_digest=? WHERE grant_id=?",
            (Bytes([3; 32]), b.grant),
        )
    });
    reject(&fixture, "G0c-worker-audit", true, true, |tx| {
        execute(
            tx,
            "UPDATE execution SET state='failed' WHERE grant_id=?",
            [b.grant],
        )
    });
    durable(
        &mut ledger,
        mutate(b, Action::WorkerFailed),
        State::Consumed,
    );
    reject(&fixture, "G0c-13", true, false, |tx| {
        execute(
            tx,
            "UPDATE grants SET state='reserved' WHERE id=?",
            [b.grant],
        )
    });
    assert_eq!(
        ledger.apply(mutate(b, Action::Consume { nonce: n })),
        Err(Error::Replay)
    );
    assert_eq!(
        fixture.inspection().execution(b.grant),
        Ok(Some(("failed".into(), false)))
    );
}

#[test]
fn g0c_writer_domain_check_rejects_valid_sql_with_wrong_transaction_context() {
    let fixture = Fixture::new();
    let mut ledger = fixture.create();
    let b = pending(&mut ledger, TARGET);
    let before = facts(&fixture);
    ledger.set_hook(Box::new(|stage, tx| {
        if stage == Stage::BeforeCommit {
            tx.execute(
                "INSERT INTO decisions VALUES(?,?,?,7,9,1)",
                (id(500), Bytes([42; 32]), id(501)),
            )?;
        }
        Ok(())
    }));
    let attempt = mutate(
        b,
        Action::Reserve {
            nonce: nonce(TARGET),
            ttl_ms: 100,
        },
    );
    assert_eq!(ledger.apply(attempt), Err(Error::Corrupt));
    assert_eq!(ledger.apply(attempt), Err(Error::Fenced));
    assert_eq!(facts(&fixture), before);
    assert_eq!(fixture.inspection().reconcile(attempt.request), Ok(None));
    println!(
        "EVIDENCE G0c-precommit domain_check=corrupt rollback=verified fence=verified independent_snapshot_sha256={before}"
    );
}

fn legacy_v2(fixture: &Fixture) -> Connection {
    File::create_new(fixture.path().join(db::DB)).unwrap();
    let raw = open_fixture(fixture, OpenFlags::SQLITE_OPEN_READ_WRITE);
    raw.execute_batch(&db::V1.replace("\r\n", "\n").replace('\n', "\r\n"))
        .unwrap();
    raw.execute_batch(include_str!("../migrations/002-events.sql"))
        .unwrap();
    raw.pragma_update(None, "application_id", db::APPLICATION_ID)
        .unwrap();
    raw.pragma_update(None, "user_version", 2).unwrap();
    raw.execute("INSERT INTO meta VALUES(1,?,7,9,0)", [id(6)])
        .unwrap();
    raw
}

fn legacy_reserve(raw: &Connection, grant: u64, consumed: bool) -> Binding {
    let b = binding(
        Epoch {
            boot: id(6),
            policy: POLICY,
            grant: GRANT_EPOCH,
        },
        grant,
    );
    raw.execute(
        "INSERT INTO decisions VALUES(?,?,?,7,9,1)",
        (b.decision, b.input, b.epoch.boot),
    )
    .unwrap();
    raw.execute(
        "INSERT INTO grants(id,decision,state) VALUES(?,?,'created')",
        (b.grant, b.decision),
    )
    .unwrap();
    raw.execute(
        "INSERT INTO audit(grant_id,kind,state) VALUES(?,'transition','created')",
        [b.grant],
    )
    .unwrap();
    raw.execute("UPDATE grants SET state='pending' WHERE id=?", [b.grant])
        .unwrap();
    raw.execute("INSERT INTO audit(grant_id,kind,previous,state) VALUES(?,'transition','created','pending')",[b.grant]).unwrap();
    raw.execute("INSERT INTO nonces VALUES(?,?)", (nonce(grant), b.grant))
        .unwrap();
    raw_reserve(raw, b.grant).unwrap();
    raw.execute("INSERT INTO audit(grant_id,kind,previous,state) VALUES(?,'transition','pending','reserved')",[b.grant]).unwrap();
    if consumed {
        raw.execute("INSERT INTO execution VALUES(?,'unknown',0)", [b.grant])
            .unwrap();
        raw.execute("UPDATE grants SET state='consumed' WHERE id=?", [b.grant])
            .unwrap();
        raw.execute("INSERT INTO audit(grant_id,kind,previous,state) VALUES(?,'transition','reserved','consumed')",[b.grant]).unwrap();
    }
    b
}

#[test]
fn g0_migration_preserves_consumed_and_invalidates_old_reserved_with_no_forged_evidence() {
    let fixture = Fixture::new();
    let raw = legacy_v2(&fixture);
    let old = legacy_reserve(&raw, TARGET, false);
    let consumed = legacy_reserve(&raw, TARGET + 1, true);
    drop(raw);
    let before = fixture.inspection();
    assert_eq!(before.grant(old.grant), Ok(Some(State::Reserved)));
    assert_eq!(before.audit(old.grant).unwrap().len(), 3);
    assert_eq!(before.grant(consumed.grant), Ok(Some(State::Consumed)));
    let consumed_audit = before.audit(consumed.grant).unwrap();
    drop(before);
    let mut ledger = fixture.reopen();
    assert_ne!(ledger.epoch().boot, old.epoch.boot);
    assert_eq!(
        ledger.apply(mutate(
            old,
            Action::Consume {
                nonce: nonce(TARGET)
            }
        )),
        Err(Error::Epoch)
    );
    drop(ledger);
    let after = fixture.inspection();
    assert_eq!(after.grant(old.grant), Ok(Some(State::Invalidated)));
    assert_eq!(after.audit(old.grant).unwrap().len(), 4);
    assert_eq!(after.grant(consumed.grant), Ok(Some(State::Consumed)));
    assert_eq!(after.audit(consumed.grant).unwrap(), consumed_audit);
    assert_eq!(
        after.execution(consumed.grant),
        Ok(Some(("unknown".into(), false)))
    );
    let read = open_fixture(&fixture, OpenFlags::SQLITE_OPEN_READ_ONLY);
    assert_eq!(db::version(&read), Ok(3));
    for (table, count) in [
        ("nonces", 2),
        ("g0_reservations", 0),
        ("g0_execution_bindings", 0),
    ] {
        assert_eq!(
            read.query_row::<i64, _, _>(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap(),
            count
        );
    }
    println!(
        "EVIDENCE G0-migration 2->3 reserved_to_invalidated audit=3->4 consumed_audit=4->4 nonces=2->2 new_binding_rows=0 independent_reopen=verified"
    );
}

#[test]
fn g0_migration_refuses_inconsistent_v2_and_keeps_original_bytes() {
    let fixture = Fixture::new();
    let raw = legacy_v2(&fixture);
    let old = legacy_reserve(&raw, TARGET, false);
    raw.execute("UPDATE grants SET state='revoked' WHERE id=?", [old.grant])
        .unwrap();
    drop(raw);
    let before = fs::read(fixture.path().join(db::DB)).unwrap();
    assert_eq!(
        Ledger::reopen(fixture.path(), POLICY, GRANT_EPOCH).err(),
        Some(Error::Corrupt)
    );
    assert_eq!(fs::read(fixture.path().join(db::DB)).unwrap(), before);
    let read = open_fixture(&fixture, OpenFlags::SQLITE_OPEN_READ_ONLY);
    assert_eq!(db::version(&read), Ok(2));
    assert_eq!(
        read.query_row::<i64, _, _>("SELECT count(*) FROM audit", [], |r| r.get(0))
            .unwrap(),
        3
    );
    println!(
        "EVIDENCE G0-migration-invalid version=2 retained=true ledger_reopen=corrupt sha256={:x}",
        Sha256::digest(before)
    );
}

#[test]
fn g0_before_fix_defects_reproduced_on_pinned_sqlite() {
    let fixture = Fixture::new();
    let raw = legacy_v2(&fixture);
    let old = legacy_reserve(&raw, TARGET, false);
    let tx = raw.unchecked_transaction().unwrap();
    execute(
        &tx,
        "INSERT INTO decisions VALUES(?,?,?,7,9,1)",
        (id(500), Bytes([4; 32]), old.epoch.boot),
    )
    .unwrap();
    execute(
        &tx,
        "UPDATE grants SET decision=?,wall_deadline=9100,mono_deadline=9200 WHERE id=?",
        (id(500), old.grant),
    )
    .unwrap();
    let actual: (Id, i64, i64) = tx
        .query_row(
            "SELECT decision,wall_deadline,mono_deadline FROM grants WHERE id=?",
            [old.grant],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(actual, (id(500), 9100, 9200));
    execute(
        &tx,
        "UPDATE grants SET state='consumed' WHERE id=?",
        [old.grant],
    )
    .unwrap();
    assert_eq!(
        tx.query_row::<i64, _, _>("SELECT count(*) FROM execution", [], |r| r.get(0))
            .unwrap(),
        0
    );
    assert_eq!(
        tx.query_row::<String, _, _>(
            "SELECT state FROM audit ORDER BY seq DESC LIMIT 1",
            [],
            |r| r.get(0)
        )
        .unwrap(),
        "reserved"
    );
    tx.commit().unwrap();
    drop(raw);
    let independent = open_fixture(&fixture, OpenFlags::SQLITE_OPEN_READ_ONLY);
    assert_eq!(db::domain_integrity(&independent), Err(Error::Corrupt));
    let persisted: (Id, i64, i64, String) = independent
        .query_row(
            "SELECT decision,wall_deadline,mono_deadline,state FROM grants WHERE id=?",
            [old.grant],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(persisted, (id(500), 9100, 9200, "consumed".into()));
    println!(
        "EVIDENCE G0-before version=3.54.1 legacy_rebind=accepted legacy_deadlines=extended legacy_consume_without_audit_or_execution=accepted independent_reopen=verified fixture=disposable"
    );
}
