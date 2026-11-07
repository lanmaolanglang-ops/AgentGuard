use super::*;
use crate::Result;
use crate::{Action, Binding, Mutation, Outcome, Writer};
use d::{s_preflight::ReserveInput, *};
use serde_json::Value;
use std::{
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

const NOW: u64 = 1_793_865_600_000;
fn uid(n: u64) -> Uuid {
    Uuid::from_u128(0x0000_0000_0000_4000_8000_0000_0000_0000 | u128::from(n))
}
fn req() -> Id {
    static N: AtomicU64 = AtomicU64::new(100_000);
    id(uid(N.fetch_add(1, Ordering::Relaxed)))
}
fn utc(n: u64) -> UtcMillis {
    UtcMillis::from_unix_millis(n).unwrap()
}
fn sample() -> ClockSample {
    ClockSample {
        os_boot_id: uid(0x500),
        utc: utc(NOW),
        monotonic_ms: 1000,
    }
}
fn keys(generation: u64) -> SyntheticKeys {
    SyntheticKeys::new(
        format!("s1-fixture-{generation}"),
        generation,
        std::array::from_fn(|i| [generation as u8 * 8 + i as u8 + 1; 32]),
    )
    .unwrap()
}
fn config(clock: &SyntheticClock, generation: u64) -> FixtureConfig {
    FixtureConfig {
        keys: keys(generation),
        clock: clock.clone(),
        nonces: SyntheticNonces::new(
            (1..=200)
                .map(|n| {
                    let mut b = [0x71; 32];
                    b[..8].copy_from_slice(&u64::to_be_bytes(n));
                    b[8] = generation as u8;
                    b
                })
                .collect(),
        ),
        startup_fault: None,
    }
}
fn fixture_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!("agentguard-ledger-{}", Uuid::new_v4()));
    fs::create_dir(&path).unwrap();
    fs::write(path.join("fixture"), db::MARKER).unwrap();
    path
}
struct Setup {
    ledger: Option<Ledger>,
    root: PathBuf,
    fence: PathBuf,
    clock: SyntheticClock,
    epoch: Epoch,
    activation: Activation,
}
impl Setup {
    fn new() -> Self {
        let root = fixture_dir();
        let fence = fixture_dir();
        let clock = SyntheticClock::new(sample());
        let ledger = Ledger::create_s(&root, &fence, 7, 9, config(&clock, 1)).unwrap();
        let epoch = ledger.epoch();
        let activation = ledger.s_activation().unwrap();
        Self {
            ledger: Some(ledger),
            root,
            fence,
            clock,
            epoch,
            activation,
        }
    }
    fn ledger(&mut self) -> &mut Ledger {
        self.ledger.as_mut().unwrap()
    }
    fn candidate(&self, n: u64, parent: Option<RuleRef>) -> PrecommitCandidate {
        let fixtures: Vec<Value> =
            serde_json::from_str(include_str!("../../../../fixtures/p0.1.json")).unwrap();
        let mut operation =
            fixtures.into_iter().find(|r| r["id"] == "OP01").unwrap()["value"].clone();
        operation["invocation_id"] = json!(uid(n + 3000).to_string());
        operation["secret_parameters"]["key_id"] =
            json!(format!("s1-fixture-{}", self.activation.generation));
        let target = agentguard_protocol::crypto::typed_encode(&operation["targets"][0]).unwrap();
        operation["handle_manifest"][0]["object_identity_digest"] =
            json!(agentguard_protocol::crypto::hex(&digest(&target).0));
        preflight_s_reserve(ReserveInput {
            grant: uid(n),
            approval: uid(n + 1000),
            approval_version: 1,
            parent,
            epochs: Epochs {
                policy: PolicyEpoch(self.epoch.policy.into()),
                grant: GrantEpoch(self.epoch.grant.into()),
                core: CoreEpoch(uuid(self.epoch.boot)),
            },
            preparation: Preparation {
                id: uid(n + 2000),
                peer: PeerBinding {
                    connection: uid(600),
                    broker_instance: uid(601),
                    worker_instance: uid(602),
                },
                generation: 1,
                deadline: Deadline {
                    utc: utc(NOW + 1_200_000),
                    monotonic_ms: 1_201_000,
                },
            },
            decision: Decision {
                id: uid(n + 4000),
                hard_deny: false,
                native_permission: NativePermission::Allow,
                identity: SafetyEvidence::Known,
                safety: SafetyEvidence::Known,
            },
            operation: serde_json::to_vec(&operation).unwrap(),
            secret_kinds: vec![],
            ttl_ms: 30_000,
        })
        .unwrap()
    }
    fn run(&mut self, command: SCommand) -> Result<SOutcome> {
        let r = req();
        self.run_id(r, command)
    }
    fn run_id(&mut self, request: Id, command: SCommand) -> Result<SOutcome> {
        let activation = self.activation.core;
        self.ledger().apply_s(SRequest {
            request,
            activation,
            command,
        })
    }
    fn register(&mut self, n: u64, parent: Option<RuleRef>) {
        let candidate = self.candidate(n, parent);
        self.run(SCommand::Policy { candidate }).unwrap();
        self.approve(n, parent, 1);
    }
    fn approve(&mut self, n: u64, parent: Option<RuleRef>, version: u32) {
        let mut input = self.candidate(n, parent).into_input();
        input.approval_version = version;
        let candidate = preflight_s_reserve(input).unwrap();
        let approval = UiApproval::confirm(
            Role::Ui,
            uid(700),
            Uuid::new_v4(),
            Uuid::new_v4(),
            &candidate,
            Deadline {
                utc: utc(NOW + 1_200_000),
                monotonic_ms: 1_201_000,
            },
        )
        .unwrap();
        self.run(SCommand::Register {
            candidate,
            approval,
        })
        .unwrap();
    }
    fn rule(&mut self, n: u64, reference: RuleRef) {
        let candidate = self.candidate(n, Some(reference));
        let approval = RuleApproval::confirm(
            Role::Ui,
            reference,
            Uuid::new_v4(),
            self.clock.sample().unwrap().utc.unix_millis(),
            3,
            600_000,
        )
        .unwrap();
        self.run(SCommand::Rule {
            candidate,
            approval,
        })
        .unwrap();
    }
    fn reserve(&mut self, n: u64, parent: Option<RuleRef>) -> Result<DurableCommitReceipt> {
        let candidate = self.candidate(n, parent);
        match self.run(SCommand::Reserve {
            candidate,
            reservation: uid(n + 5000),
        })? {
            SOutcome::Fresh(receipt) => Ok(receipt),
            _ => panic!("fresh reserve"),
        }
    }
    fn wire(&mut self, n: u64, parent: Option<RuleRef>) -> Vec<u8> {
        let peer = self.candidate(n, parent).input().preparation.peer;
        let receipt = self.reserve(n, parent).unwrap();
        match self.ledger().take_s(receipt, peer).unwrap() {
            SResponse::Permit(p) => p.into_wire(),
            _ => panic!("permit"),
        }
    }
    fn consume(&mut self, n: u64, parent: Option<RuleRef>, wire: &[u8]) -> Result<SOutcome> {
        let candidate = preflight_s_consume(self.candidate(n, parent), wire).unwrap();
        self.run(SCommand::Consume {
            candidate,
            launch: uid(n + 6000),
        })
    }
    fn inspect(&self, label: &str) -> Value {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let out = observations_dir(repo).join(format!("{label}-{}.json", Uuid::new_v4()));
        let python = std::env::var_os("AG_S1_PYTHON").unwrap_or("python".into());
        let output = Command::new(python)
            .arg("-B")
            .arg(repo.join("tools/inspect_i06_s.py"))
            .arg(&self.root)
            .arg("--fence")
            .arg(&self.fence)
            .arg("--output")
            .arg(&out)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        println!("S1_EVIDENCE {label} {}", out.display());
        result
    }
    fn inject(&mut self, point: FaultPoint) {
        self.ledger().s.as_mut().unwrap().hook = Some(Box::new(move |p, _| {
            if p == point {
                Err(Error::Input)
            } else {
                Ok(())
            }
        }));
    }
    fn restart(&mut self, generation: u64) {
        drop(self.ledger.take());
        let ledger = Ledger::reopen_s(
            &self.root,
            &self.fence,
            self.epoch.policy,
            self.epoch.grant + 1,
            config(&self.clock, generation),
        )
        .unwrap();
        self.activation = ledger.s_activation().unwrap();
        self.epoch = ledger.epoch();
        self.ledger = Some(ledger);
    }
}

fn observations_dir(repo: &Path) -> PathBuf {
    std::env::var_os("AG_S1_OBSERVATIONS")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join(".work/i06-s/observations"))
}

#[test]
fn f00_control_reserve_consume_reconcile() {
    let mut s = Setup::new();
    s.register(10, None);
    let wire = s.wire(10, None);
    let result = s.consume(10, None, &wire).unwrap();
    let SOutcome::Fresh(receipt) = result else {
        panic!("fresh")
    };
    let request = receipt.view.request;
    let peer = s.candidate(10, None).input().preparation.peer;
    let SResponse::Launch(launch) = s.ledger().take_s(receipt, peer).unwrap() else {
        panic!("launch")
    };
    assert_eq!(launch.metadata().grant, id(uid(10)));
    assert!(matches!(
        crate::Inspection::open(&s.root)
            .unwrap()
            .reconcile_s(request, id(uid(10))),
        ReconcileView::Committed {
            execution_unknown: true,
            ..
        }
    ));
    let facts = s.inspect("F00-control");
    assert_eq!(facts["counts"]["s_launch_once"], 1);
}

#[test]
fn f01_approval_missing_version_source_cross_grant() {
    for variant in 0..4 {
        let mut s = Setup::new();
        if variant != 0 {
            s.register(10, None);
        }
        match variant {
            0 => assert!(s.reserve(10, None).is_err()),
            1 => {
                s.approve(10, None, 2);
                assert!(s.reserve(10, None).is_err());
            }
            2 => {
                let candidate = s.candidate(11, None);
                assert!(
                    UiApproval::confirm(
                        Role::Adapter,
                        uid(700),
                        uid(701),
                        uid(702),
                        &candidate,
                        candidate.input().preparation.deadline
                    )
                    .is_err()
                );
            }
            _ => {
                s.register(11, None);
                let mut input = s.candidate(11, None).into_input();
                input.approval = uid(1010);
                let candidate = preflight_s_reserve(input).unwrap();
                assert!(
                    s.run(SCommand::Reserve {
                        candidate,
                        reservation: uid(5011)
                    })
                    .is_err()
                );
            }
        }
        assert_eq!(
            s.inspect(&format!("F01-{variant}"))["counts"]["s_approval_claims"],
            0
        );
    }
}

#[test]
fn f02_rule_version_revocation_expiry_no_downgrade() {
    for variant in 0..4 {
        let mut s = Setup::new();
        let r = RuleRef {
            id: uid(900),
            version: 1,
        };
        s.rule(10, r);
        s.register(10, Some(r));
        match variant {
            0 => s.rule(10, RuleRef { version: 2, ..r }),
            1 => {
                s.run(SCommand::RevokeRule { rule: r.id }).unwrap();
            }
            2 => {
                s.clock.set(Some(ClockSample {
                    utc: utc(NOW + 600_000),
                    monotonic_ms: 601_000,
                    ..sample()
                }));
            }
            _ => {}
        }
        assert!(
            s.reserve(10, if variant == 3 { None } else { Some(r) })
                .is_err()
        );
        assert_eq!(s.inspect(&format!("F02-{variant}"))["used"], json!([0]));
    }
}

#[test]
fn f03_hundred_requests_last_quota() {
    let mut s = Setup::new();
    let r = RuleRef {
        id: uid(900),
        version: 1,
    };
    s.rule(10, r);
    for n in 10..112 {
        s.register(n, Some(r));
    }
    s.wire(10, Some(r));
    s.wire(11, Some(r));
    let requests: Vec<_> = (12..112)
        .map(|n| SRequest {
            request: req(),
            activation: s.activation.core,
            command: SCommand::Reserve {
                candidate: s.candidate(n, Some(r)),
                reservation: uid(n + 5000),
            },
        })
        .collect();
    let writer = Writer::start(s.ledger.take().unwrap()).unwrap();
    let client = writer.client();
    let barrier = Arc::new(std::sync::Barrier::new(100));
    let joins: Vec<_> = requests
        .into_iter()
        .map(|request| {
            let client = client.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                client.submit_s(request).unwrap().recv().unwrap()
            })
        })
        .collect();
    let results: Vec<_> = joins.into_iter().map(|j| j.join().unwrap()).collect();
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Ok(SOutcome::Fresh(_))))
            .count(),
        1
    );
    assert_eq!(results.iter().filter(|r| r.is_err()).count(), 99);
    let winner = results
        .into_iter()
        .find_map(|result| match result {
            Ok(SOutcome::Fresh(receipt)) => Some(receipt),
            _ => None,
        })
        .unwrap();
    let peer = s.candidate(10, Some(r)).input().preparation.peer;
    assert!(matches!(
        client
            .take_s(winner, peer)
            .unwrap()
            .recv()
            .unwrap()
            .unwrap(),
        SResponse::Permit(_)
    ));
    drop(client);
    writer.join().unwrap();
    let facts = s.inspect("F03-100-last-quota");
    assert_eq!(facts["used"], json!([3]));
    assert_eq!(facts["counts"]["s_reservations"], 3);
}

#[test]
fn f04_nonce_collision_bound_and_request_conflicts() {
    for continuous in [false, true] {
        let mut s = Setup::new();
        s.register(10, None);
        s.register(11, None);
        s.ledger().s.as_mut().unwrap().nonces = SyntheticNonces::new(vec![
            [0x41; 32],
            [0x41; 32],
            if continuous { [0x41; 32] } else { [0x42; 32] },
            [0x43; 32],
        ]);
        s.wire(10, None);
        let before = s.ledger().s.as_ref().unwrap().nonces.attempts;
        let result = s.reserve(11, None);
        assert_eq!(result.is_err(), continuous);
        assert_eq!(s.ledger().s.as_ref().unwrap().nonces.attempts - before, 2);
        let facts = s.inspect(if continuous {
            "F04-two-collisions"
        } else {
            "F04-collision-retry"
        });
        assert_eq!(
            facts["counts"]["s_reservations"],
            if continuous { 1 } else { 2 }
        );
    }
    let mut s = Setup::new();
    s.register(10, None);
    s.register(11, None);
    let candidate = s.candidate(10, None);
    let request = req();
    s.run_id(
        request,
        SCommand::Reserve {
            candidate,
            reservation: uid(5010),
        },
    )
    .unwrap();
    let candidate = s.candidate(11, None);
    assert!(
        s.run_id(
            request,
            SCommand::Reserve {
                candidate,
                reservation: uid(5011)
            }
        )
        .is_err()
    );
    let candidate = s.candidate(11, None);
    assert!(
        s.run(SCommand::Reserve {
            candidate,
            reservation: uid(5010)
        })
        .is_err()
    );
    assert_eq!(s.inspect("F04-id-conflicts")["counts"]["s_reservations"], 1);
}

#[test]
fn f05_stale_candidate_rechecks_writer_state() {
    for variant in 0..3 {
        let mut s = Setup::new();
        s.register(10, None);
        let candidate = s.candidate(10, None);
        let command = match variant {
            0 => SCommand::RevokeApproval {
                approval: uid(1010),
            },
            1 => SCommand::InvalidatePrepared {
                prepared: uid(2010),
            },
            _ => SCommand::Cancel { grant: uid(10) },
        };
        let writer = Writer::start(s.ledger.take().unwrap()).unwrap();
        let client = writer.client();
        let first = client
            .submit_s(SRequest {
                request: req(),
                activation: s.activation.core,
                command,
            })
            .unwrap();
        let second = client
            .submit_s(SRequest {
                request: req(),
                activation: s.activation.core,
                command: SCommand::Reserve {
                    candidate,
                    reservation: uid(5010),
                },
            })
            .unwrap();
        first.recv().unwrap().unwrap();
        assert!(second.recv().unwrap().is_err());
        drop(client);
        writer.join().unwrap();
        assert_eq!(
            s.inspect(&format!("F05-{variant}"))["counts"]["s_approval_claims"],
            0
        );
    }
}

#[test]
fn f06_reserve_every_write_rolls_back() {
    for point in [
        FaultPoint::Claim,
        FaultPoint::Quota,
        FaultPoint::G0Reservation,
        FaultPoint::Nonce,
        FaultPoint::Reservation,
        FaultPoint::State,
        FaultPoint::G0Audit,
        FaultPoint::Audit,
        FaultPoint::Receipt,
        FaultPoint::BeforeCommit,
    ] {
        let mut s = Setup::new();
        let r = RuleRef {
            id: uid(900),
            version: 1,
        };
        s.rule(10, r);
        s.register(10, Some(r));
        let before = s.inspect(&format!("F06-{point:?}-before"));
        s.inject(point);
        assert!(s.reserve(10, Some(r)).is_err());
        let after = s.inspect(&format!("F06-{point:?}-after"));
        assert_eq!(before["counts"], after["counts"]);
        assert_eq!(after["used"], json!([0]));
        assert_eq!(after["states"]["pending"], 1);
    }
}

fn crash_case(label: &str, consume: bool, after: bool) {
    let s = Setup::new();
    let root = s.root.clone();
    let fence = s.fence.clone();
    drop(s.ledger);
    let ready = root.join("crash-ready");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "store::s::tests::process_probe", "--nocapture"])
        .env("AG_S1_CRASH_ROOT", &root)
        .env("AG_S1_CRASH_FENCE", &fence)
        .env("AG_S1_CRASH_CONSUME", if consume { "1" } else { "0" })
        .env("AG_S1_CRASH_AFTER", if after { "1" } else { "0" })
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let end = Instant::now() + Duration::from_secs(45);
    while !ready.exists() && Instant::now() < end {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("crash probe exited {status}");
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(ready.exists());
    child.kill().unwrap();
    let exit = child.wait().unwrap();
    assert!(!exit.success());
    println!("S1_PROCESS {label} forced_termination={exit} after_commit={after} consume={consume}");
    let clock = SyntheticClock::new(sample());
    let ledger = Ledger::reopen_s(&root, &fence, 7, 11, config(&clock, 3)).unwrap();
    let s = Setup {
        epoch: ledger.epoch(),
        activation: ledger.s_activation().unwrap(),
        ledger: Some(ledger),
        root,
        fence,
        clock,
    };
    let result = s.inspect(label);
    let view = crate::Inspection::open(&s.root)
        .unwrap()
        .reconcile_s(id(uid(88_888)), id(uid(10)));
    assert_eq!(matches!(view, ReconcileView::Committed { .. }), after);
    if !after {
        assert_eq!(view, ReconcileView::NotObserved);
    }
    assert_eq!(
        result["counts"]["s_reservations"],
        if consume || after { 1 } else { 0 }
    );
    assert_eq!(
        result["counts"]["s_launch_once"],
        if consume && after { 1 } else { 0 }
    );
    assert_eq!(
        result["states"][if consume && after {
            "consumed"
        } else {
            "invalidated"
        }],
        1
    );
}

#[test]
fn process_probe() {
    let Some(root) = std::env::var_os("AG_S1_CRASH_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let fence = PathBuf::from(std::env::var_os("AG_S1_CRASH_FENCE").unwrap());
    let clock = SyntheticClock::new(sample());
    let ledger = Ledger::reopen_s(&root, &fence, 7, 10, config(&clock, 2)).unwrap();
    let mut s = Setup {
        epoch: ledger.epoch(),
        activation: ledger.s_activation().unwrap(),
        ledger: Some(ledger),
        root,
        fence,
        clock,
    };
    s.register(10, None);
    let consume = std::env::var("AG_S1_CRASH_CONSUME").unwrap() == "1";
    let wire = if consume {
        Some(s.wire(10, None))
    } else {
        None
    };
    let after = std::env::var("AG_S1_CRASH_AFTER").unwrap() == "1";
    let ready = s.root.join("crash-ready");
    s.ledger().s.as_mut().unwrap().hook = Some(Box::new(move |point, _| {
        if point
            == if after {
                FaultPoint::AfterCommit
            } else {
                FaultPoint::BeforeCommit
            }
        {
            fs::write(&ready, b"writer reached fault boundary").unwrap();
            loop {
                thread::sleep(Duration::from_millis(100));
            }
        }
        Ok(())
    }));
    let candidate = s.candidate(10, None);
    let command = if let Some(wire) = wire {
        SCommand::Consume {
            candidate: preflight_s_consume(candidate, &wire).unwrap(),
            launch: uid(6010),
        }
    } else {
        SCommand::Reserve {
            candidate,
            reservation: uid(5010),
        }
    };
    let writer = Writer::start(s.ledger.take().unwrap()).unwrap();
    let _ = writer
        .client()
        .submit_s(SRequest {
            request: id(uid(88_888)),
            activation: s.activation.core,
            command,
        })
        .unwrap()
        .recv();
    panic!("fault boundary was not reached");
}
#[test]
fn f07_reserve_precommit_process_kill() {
    crash_case("F07-killed", false, false);
}
#[test]
fn f08_reserve_postcommit_process_kill() {
    crash_case("F08-killed", false, true);
}

#[test]
fn f09_storage_busy_full_and_commit_unknown() {
    for (point, error) in [
        (FaultPoint::Claim, Error::Busy),
        (FaultPoint::Quota, Error::Sqlite(13)),
        (FaultPoint::CommitUnknown, Error::UnknownCommit),
    ] {
        let mut s = Setup::new();
        s.register(10, None);
        s.ledger().s.as_mut().unwrap().hook =
            Some(Box::new(
                move |p, _| if p == point { Err(error) } else { Ok(()) },
            ));
        assert!(s.reserve(10, None).is_err());
        assert!(s.ledger().fenced);
        assert_eq!(
            s.inspect(&format!("F09-{point:?}"))["counts"]["s_reservations"],
            0
        );
    }
    // Real SQLite BEGIN IMMEDIATE contention, in addition to deterministic error injection.
    let mut s = Setup::new();
    s.register(10, None);
    let blocker = db::connection(&s.root.join(db::DB), OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert_eq!(s.reserve(10, None).err(), Some(Error::Busy));
    blocker.execute_batch("ROLLBACK").unwrap();
    assert_eq!(s.inspect("F09-real-busy")["counts"]["s_reservations"], 0);
    let mut s = Setup::new();
    s.register(10, None);
    s.ledger().s.as_mut().unwrap().hook = Some(Box::new(|point, tx| {
        if point == FaultPoint::CommitUnknown {
            tx.execute_batch("PRAGMA defer_foreign_keys=ON")?;
            tx.execute(
                "INSERT INTO s_key_history VALUES(?,999)",
                [Bytes([99u8; 32])],
            )?;
        }
        Ok(())
    }));
    assert_eq!(s.reserve(10, None).err(), Some(Error::UnknownCommit));
    assert_eq!(s.reserve(10, None).err(), Some(Error::Fenced));
    assert_eq!(
        s.inspect("F09-real-commit-error")["counts"]["s_reservations"],
        0
    );
}

#[test]
fn f10_disconnected_original_reply_no_reissue() {
    let mut s = Setup::new();
    s.register(10, None);
    let request = req();
    let candidate = s.candidate(10, None);
    let writer = Writer::start(s.ledger.take().unwrap()).unwrap();
    let client = writer.client();
    let first = client
        .submit_s(SRequest {
            request,
            activation: s.activation.core,
            command: SCommand::Reserve {
                candidate,
                reservation: uid(5010),
            },
        })
        .unwrap();
    drop(first);
    let candidate = s.candidate(10, None);
    assert!(matches!(
        client
            .submit_s(SRequest {
                request,
                activation: s.activation.core,
                command: SCommand::Reserve {
                    candidate,
                    reservation: uid(5010)
                }
            })
            .unwrap()
            .recv()
            .unwrap()
            .unwrap(),
        SOutcome::StatusOnly(_)
    ));
    drop(client);
    writer.join().unwrap();
    assert_eq!(s.inspect("F10-disconnect")["counts"]["s_reservations"], 1);
}

#[test]
fn f11_final_macs_peer_and_snapshot() {
    for variant in 0..7 {
        let mut s = Setup::new();
        s.register(10, None);
        let wire = s.wire(10, None);
        let mut value: Value = serde_json::from_slice(&wire).unwrap();
        let mut input = s.candidate(10, None).into_input();
        match variant {
            0 => value["mac"] = json!("00".repeat(32)),
            1 => {
                value["operation_digest"] = json!("00".repeat(32));
                value.as_object_mut().unwrap().remove("mac");
                let mac = agentguard_protocol::crypto::mac(
                    &[13; 32],
                    agentguard_protocol::crypto::Domain::Permit,
                    &value,
                )
                .unwrap();
                value["mac"] = json!(agentguard_protocol::crypto::hex(&mac));
            }
            2 => value["key_id"] = json!("other-key"),
            3 => value["reservation_id"] = json!(uid(5999).to_string()),
            4 => input.preparation.peer.connection = uid(666),
            5 => input.preparation.generation = 2,
            _ => {
                let mut op: Value = serde_json::from_slice(&input.operation).unwrap();
                op["config_digest"] = json!("22".repeat(32));
                input.operation = serde_json::to_vec(&op).unwrap();
            }
        }
        let context = preflight_s_reserve(input).unwrap();
        let candidate = preflight_s_consume(context, &serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(
            s.run(SCommand::Consume {
                candidate,
                launch: uid(6010)
            })
            .is_err()
        );
        assert_eq!(
            s.inspect(&format!("F11-{variant}"))["counts"]["s_launch_once"],
            0
        );
    }
}

#[test]
fn f12_dual_deadlines_clock_errors_and_late_response() {
    for variant in 0..6 {
        let mut s = Setup::new();
        s.register(10, None);
        let wire = s.wire(10, None);
        let mut now = sample();
        match variant {
            0 => now.utc = utc(NOW + 30_000),
            1 => now.monotonic_ms = 31_000,
            2 => now.utc = utc(NOW - 1),
            3 => now.monotonic_ms = u64::MAX,
            _ => {}
        }
        s.clock.set(if variant == 4 { None } else { Some(now) });
        if variant == 5 {
            let clock = s.clock.clone();
            s.ledger().s.as_mut().unwrap().hook = Some(Box::new(move |point, _| {
                if point == FaultPoint::AfterCommit {
                    clock.set(Some(ClockSample {
                        utc: utc(NOW + 30_000),
                        ..sample()
                    }));
                }
                Ok(())
            }));
            let SOutcome::Fresh(receipt) = s.consume(10, None, &wire).unwrap() else {
                panic!("fresh")
            };
            let peer = s.candidate(10, None).input().preparation.peer;
            assert!(s.ledger().take_s(receipt, peer).is_err());
        } else {
            assert!(s.consume(10, None, &wire).is_err());
        }
        let facts = s.inspect(&format!("F12-{variant}"));
        assert_eq!(
            facts["counts"]["s_launch_once"],
            if variant == 5 { 1 } else { 0 }
        );
    }
}

#[test]
fn f13_hundred_same_nonce_consumers() {
    let mut s = Setup::new();
    s.register(10, None);
    let wire = s.wire(10, None);
    let shared = req();
    let requests: Vec<_> = (0..100)
        .map(|n| SRequest {
            request: if n < 50 { shared } else { req() },
            activation: s.activation.core,
            command: SCommand::Consume {
                candidate: preflight_s_consume(s.candidate(10, None), &wire).unwrap(),
                launch: uid(6010),
            },
        })
        .collect();
    let writer = Writer::start(s.ledger.take().unwrap()).unwrap();
    let client = writer.client();
    let barrier = Arc::new(std::sync::Barrier::new(100));
    let joins: Vec<_> = requests
        .into_iter()
        .map(|request| {
            let client = client.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                client.submit_s(request).unwrap().recv().unwrap()
            })
        })
        .collect();
    let mut fresh = Vec::new();
    for join in joins {
        if let Ok(SOutcome::Fresh(receipt)) = join.join().unwrap() {
            fresh.push(receipt);
        }
    }
    assert_eq!(fresh.len(), 1);
    let peer = s.candidate(10, None).input().preparation.peer;
    assert!(matches!(
        client
            .take_s(fresh.pop().unwrap(), peer)
            .unwrap()
            .recv()
            .unwrap()
            .unwrap(),
        SResponse::Launch(_)
    ));
    drop(client);
    writer.join().unwrap();
    assert_eq!(s.inspect("F13-100-consumers")["counts"]["s_launch_once"], 1);
}

#[test]
fn f14_consume_every_write_rolls_back() {
    for point in [
        FaultPoint::State,
        FaultPoint::Execution,
        FaultPoint::Launch,
        FaultPoint::Facts,
        FaultPoint::G0Audit,
        FaultPoint::Audit,
        FaultPoint::Receipt,
        FaultPoint::BeforeCommit,
    ] {
        let mut s = Setup::new();
        s.register(10, None);
        let wire = s.wire(10, None);
        let before = s.inspect(&format!("F14-{point:?}-before"));
        s.inject(point);
        assert!(s.consume(10, None, &wire).is_err());
        let after = s.inspect(&format!("F14-{point:?}-after"));
        assert_eq!(before["counts"], after["counts"]);
        assert_eq!(after["states"]["reserved"], 1);
    }
}
#[test]
fn f15_consume_precommit_process_kill() {
    crash_case("F15-killed", true, false);
}
#[test]
fn f16_consume_postcommit_process_kill() {
    crash_case("F16-killed", true, true);
}

#[test]
fn f17_revoke_and_response_take_order() {
    for revoke_first in [false, true] {
        let mut s = Setup::new();
        s.register(10, None);
        let wire = s.wire(10, None);
        if revoke_first {
            s.run(SCommand::RevokeApproval {
                approval: uid(1010),
            })
            .unwrap();
            assert!(s.consume(10, None, &wire).is_err());
        } else {
            let SOutcome::Fresh(receipt) = s.consume(10, None, &wire).unwrap() else {
                panic!("fresh")
            };
            s.run(SCommand::Cancel { grant: uid(10) }).unwrap();
            let peer = s.candidate(10, None).input().preparation.peer;
            assert!(s.ledger().take_s(receipt, peer).is_err());
        }
        let facts = s.inspect(if revoke_first {
            "F17-revoke-first"
        } else {
            "F17-consume-first"
        });
        assert_eq!(
            facts["counts"]["s_launch_once"],
            if revoke_first { 0 } else { 1 }
        );
    }
    let mut s = Setup::new();
    s.register(10, None);
    let receipt = s.reserve(10, None).unwrap();
    s.run(SCommand::RevokeApproval {
        approval: uid(1010),
    })
    .unwrap();
    let peer = s.candidate(10, None).input().preparation.peer;
    assert!(s.ledger().take_s(receipt, peer).is_err());
    s.inspect("F17-reserve-egress-revoked");

    let mut s = Setup::new();
    s.register(10, None);
    let receipt = s.reserve(10, None).unwrap();
    let peer = s.candidate(10, None).input().preparation.peer;
    let writer = Writer::start(s.ledger.take().unwrap()).unwrap();
    let client = writer.client();
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let take = thread::spawn({
        let client = client.clone();
        let barrier = barrier.clone();
        move || {
            barrier.wait();
            client.take_s(receipt, peer).unwrap().recv().unwrap()
        }
    });
    let revoke = thread::spawn({
        let client = client.clone();
        let activation = s.activation.core;
        move || {
            barrier.wait();
            client
                .submit_s(SRequest {
                    request: req(),
                    activation,
                    command: SCommand::RevokeApproval {
                        approval: uid(1010),
                    },
                })
                .unwrap()
                .recv()
                .unwrap()
                .unwrap()
        }
    });
    revoke.join().unwrap();
    let taken = take.join().unwrap();
    println!("S1_EGRESS F17 competing_take_delivered={}", taken.is_ok());
    match taken {
        Ok(SResponse::Permit(permit)) => {
            let candidate =
                preflight_s_consume(s.candidate(10, None), &permit.into_wire()).unwrap();
            assert!(
                client
                    .submit_s(SRequest {
                        request: req(),
                        activation: s.activation.core,
                        command: SCommand::Consume {
                            candidate,
                            launch: uid(6010)
                        }
                    })
                    .unwrap()
                    .recv()
                    .unwrap()
                    .is_err()
            );
        }
        Err(Error::Replay) => {}
        _ => panic!("unexpected response race outcome"),
    }
    drop(client);
    writer.join().unwrap();
    let facts = s.inspect("F17-egress-race");
    assert_eq!(facts["counts"]["s_reservations"], 1);
    assert_eq!(facts["counts"]["s_launch_once"], 0);
}

#[test]
fn f18_legacy_guard_and_global_request_namespace() {
    let mut s = Setup::new();
    s.register(10, None);
    let candidate = s.candidate(10, None);
    let binding = Binding {
        grant: id(uid(10)),
        decision: id(candidate.input().decision.id),
        input: Bytes(candidate.fingerprint()),
        epoch: s.epoch,
    };
    assert_eq!(
        s.ledger()
            .apply(Mutation {
                request: req(),
                binding,
                action: Action::Reserve {
                    nonce: Bytes([5; 32]),
                    ttl_ms: 1000
                }
            })
            .err(),
        Some(Error::Scope)
    );
    let legacy = Binding {
        grant: id(uid(90_000)),
        decision: id(uid(90_001)),
        input: Bytes([7; 32]),
        epoch: s.epoch,
    };
    let request = req();
    assert!(matches!(
        s.ledger()
            .apply(Mutation {
                request,
                binding: legacy,
                action: Action::Create { eligible: true }
            })
            .unwrap(),
        Outcome::Durable(_)
    ));
    let candidate = s.candidate(10, None);
    assert!(
        s.run_id(
            request,
            SCommand::Reserve {
                candidate,
                reservation: uid(5010)
            }
        )
        .is_err()
    );
    let receipt = s.reserve(10, None).unwrap();
    assert_eq!(
        s.ledger()
            .apply(Mutation {
                request: receipt.view.request,
                binding: legacy,
                action: Action::Pend
            })
            .err(),
        Some(Error::Scope)
    );
    s.inspect("F18-legacy-global-namespace");
}

#[test]
fn f19_restart_rotation_history_and_quota() {
    let mut s = Setup::new();
    let r = RuleRef {
        id: uid(900),
        version: 1,
    };
    s.rule(10, r);
    s.register(10, Some(r));
    s.register(11, Some(r));
    let wire = s.wire(10, Some(r));
    s.consume(10, Some(r), &wire).unwrap();
    s.wire(11, Some(r));
    let previous = s.activation;
    s.restart(2);
    assert_ne!(previous.core, s.activation.core);
    assert_ne!(previous.clock_domain, s.activation.clock_domain);
    assert!(s.consume(10, Some(r), &wire).is_err());
    let facts = s.inspect("F19-rotation");
    assert_eq!(facts["states"]["consumed"], 1);
    assert_eq!(facts["states"]["invalidated"], 1);
    assert_eq!(facts["used"], json!([2]));
    s.rule(12, RuleRef { version: 2, ..r });
    s.register(12, Some(RuleRef { version: 2, ..r }));
    s.wire(12, Some(RuleRef { version: 2, ..r }));
    assert_eq!(s.inspect("F19-rule-version-history")["used"], json!([3]));
}

#[test]
fn f20_os_boot_and_clock_domain_closed() {
    let mut s = Setup::new();
    s.register(10, None);
    let wire = s.wire(10, None);
    s.clock.set(Some(ClockSample {
        os_boot_id: uid(999),
        ..sample()
    }));
    assert!(s.consume(10, None, &wire).is_err());
    assert!(s.ledger().fenced);
    s.inspect("F20-boot-mismatch");
}

#[test]
fn f21_fence_missing_replaced_and_ahead() {
    for variant in 0..3 {
        let mut s = Setup::new();
        s.register(10, None);
        drop(s.ledger.take());
        let path = s.fence.join("s-fence");
        match variant {
            0 => {
                fs::remove_file(&path).unwrap();
            }
            1 => fs::write(&path, b"replaced").unwrap(),
            _ => fs::write(
                &path,
                marker_bytes(Activation {
                    sequence: s.activation.sequence + 1,
                    ..s.activation
                }),
            )
            .unwrap(),
        }
        assert!(Ledger::reopen_s(&s.root, &s.fence, 7, 10, config(&s.clock, 2)).is_err());
        s.inspect(&format!("F21-{variant}"));
    }
    for fault in [ActivationFault::AfterMarker, ActivationFault::AfterCommit] {
        let mut s = Setup::new();
        s.register(10, None);
        drop(s.ledger.take());
        let mut cfg = config(&s.clock, 2);
        cfg.startup_fault = Some(fault);
        assert!(Ledger::reopen_s(&s.root, &s.fence, 7, 10, cfg).is_err());
        let facts = s.inspect(if matches!(fault, ActivationFault::AfterMarker) {
            "F21-after-marker"
        } else {
            "F21-after-db-commit"
        });
        if matches!(fault, ActivationFault::AfterMarker) {
            assert_eq!(facts["fence"]["status"], "mismatch");
            assert!(Ledger::reopen_s(&s.root, &s.fence, 7, 11, config(&s.clock, 3)).is_err());
        } else {
            assert_eq!(facts["fence"]["status"], "matched");
            let ledger = Ledger::reopen_s(&s.root, &s.fence, 7, 11, config(&s.clock, 3)).unwrap();
            assert_eq!(ledger.s_activation().unwrap().sequence, 3);
        }
    }
}

#[test]
fn f22_old_backup_and_lineage_lock() {
    let mut s = Setup::new();
    s.register(10, None);
    s.wire(10, None);
    assert!(Ledger::reopen_s(&s.root, &s.fence, 7, 10, config(&s.clock, 2)).is_err());
    let copy = fixture_dir();
    let mut destination = db::connection(
        &{
            let p = copy.join(db::DB);
            File::create_new(&p).unwrap();
            p
        },
        OpenFlags::SQLITE_OPEN_READ_WRITE,
    )
    .unwrap();
    {
        let backup =
            rusqlite::backup::Backup::new(&s.ledger().connection, &mut destination).unwrap();
        backup
            .run_to_completion(100, Duration::from_millis(1), None)
            .unwrap();
    }
    drop(destination);
    // A copied DB cannot activate while the lineage owner holds its independent lock.
    assert!(Ledger::reopen_s(&copy, &s.fence, 7, 10, config(&s.clock, 2)).is_err());
    s.restart(2);
    drop(s.ledger.take());
    assert!(Ledger::reopen_s(&copy, &s.fence, 7, 11, config(&s.clock, 3)).is_err());
    assert!(matches!(
        crate::Inspection::open(&copy)
            .unwrap()
            .reconcile_s(req(), id(uid(10))),
        ReconcileView::NotObserved
    ));
    s.inspect("F22-live-history");
    let original = std::mem::replace(&mut s.root, copy);
    s.inspect("F22-old-backup");
    s.root = original;
}

#[test]
fn f23_v3_v4_schema_and_independent_rollback() {
    let root = fixture_dir();
    let ledger = Ledger::create(&root, 7, 9).unwrap();
    drop(ledger);
    let before = fs::read(root.join(db::DB)).unwrap();
    let source_hash = digest(&before);
    let copy = fixture_dir();

    let fence = fixture_dir();
    let clock = SyntheticClock::new(sample());
    let v4 = Ledger::migrate_s_copy(&root, &copy, &fence, config(&clock, 1)).unwrap();
    assert_eq!(source_hash, digest(&fs::read(root.join(db::DB)).unwrap()));
    let snapshot = Setup {
        epoch: v4.epoch(),
        activation: v4.s_activation().unwrap(),
        ledger: Some(v4),
        root: copy,
        fence,
        clock,
    };
    snapshot.inspect("F23-v3-independent-v4-copy");
    let mut s = Setup::new();
    drop(s.ledger.take());
    assert_eq!(Ledger::reopen(&s.root, 7, 9).err(), Some(Error::Version));
    let ledger = Ledger::reopen(&root, 7, 9).unwrap();
    assert_eq!(db::version(&ledger.connection).unwrap(), 3);
    drop(ledger);
    assert!(crate::Inspection::open(&s.root).unwrap().verify().is_ok());
    assert!(!before.is_empty());
    let drift = db::connection(&s.root.join(db::DB), OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();
    drift
        .execute_batch("CREATE TABLE drift(id INTEGER)")
        .unwrap();
    drop(drift);
    assert!(Ledger::reopen_s(&s.root, &s.fence, 7, 10, config(&s.clock, 2)).is_err());
    // DDL + user_version remain one SQLite transaction on a separate migration fixture.
    let root = fixture_dir();
    let mut ledger = Ledger::create(&root, 7, 9).unwrap();
    {
        let tx = ledger
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .unwrap();
        tx.execute_batch(db::V4).unwrap();
        tx.pragma_update(None, "user_version", 4).unwrap();
        tx.rollback().unwrap();
    }
    assert_eq!(db::version(&ledger.connection).unwrap(), 3);
    db::integrity(&ledger.connection).unwrap();
    let fence = fixture_dir();
    let clock = SyntheticClock::new(sample());
    let mut cfg = config(&clock, 1);
    cfg.startup_fault = Some(ActivationFault::BeforeCommit);
    assert!(ledger.enable_s(&fence, cfg).is_err());
    let read = crate::Inspection::open(&root).unwrap();
    read.verify().unwrap();
    assert_eq!(db::version(&read.0).unwrap(), 3);
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let out = observations_dir(repo).join("F23-migration-rollback.json");
    let python = std::env::var_os("AG_S1_PYTHON").unwrap_or("python".into());
    let result = Command::new(python)
        .arg("-B")
        .arg(repo.join("tools/inspect_i06_s.py"))
        .arg(&root)
        .arg("--output")
        .arg(&out)
        .output()
        .unwrap();
    assert!(result.status.success());
    println!("S1_EVIDENCE F23 {}", out.display());

    let root = fixture_dir();
    let fence = fixture_dir();
    let mut cfg = config(&clock, 1);
    cfg.startup_fault = Some(ActivationFault::AfterCommit);
    assert_eq!(
        Ledger::create_s(&root, &fence, 7, 9, cfg).err(),
        Some(Error::UnknownCommit)
    );
    let ledger = Ledger::reopen_s(&root, &fence, 7, 10, config(&clock, 2)).unwrap();
    let s = Setup {
        epoch: ledger.epoch(),
        activation: ledger.s_activation().unwrap(),
        ledger: Some(ledger),
        root,
        fence,
        clock,
    };
    let facts = s.inspect("F23-migration-commit-confirmation-lost");
    assert_eq!(facts["version"], 4);
    assert_eq!(facts["activation"], 2);
}

#[test]
fn f24_reconcile_scope_unknown_unavailable_and_redaction() {
    let mut s = Setup::new();
    s.register(10, None);
    let receipt = s.reserve(10, None).unwrap();
    let request = receipt.view.request;
    let read = crate::Inspection::open(&s.root).unwrap();
    assert_eq!(
        read.reconcile_s(request, id(uid(11))),
        ReconcileView::Unavailable
    );
    assert_eq!(
        read.reconcile_s(req(), id(uid(10))),
        ReconcileView::NotObserved
    );
    let view = read.reconcile_s(request, id(uid(10)));
    let text = format!("{view:?}");
    for forbidden in [
        "nonce",
        "permit_mac",
        "operation_mac",
        "secret",
        "LaunchAuthorization",
        "key_id",
    ] {
        assert!(!text.contains(forbidden));
    }
    s.inspect("F24-reconcile-redacted");
    let corrupt = fixture_dir();
    File::create_new(corrupt.join(db::DB)).unwrap();
    assert!(crate::Inspection::open(&corrupt).is_err());
    let db = &s.ledger().connection;
    db.execute_batch("PRAGMA foreign_keys=OFF; DROP TRIGGER s_audit_immutable_delete; DELETE FROM s_audit WHERE kind='reserve'").unwrap();
    assert_eq!(
        read.reconcile_s(request, id(uid(10))),
        ReconcileView::Unknown
    );
}
