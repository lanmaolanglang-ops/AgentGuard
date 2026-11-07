use super::*;
use crate::{Action, AuditKind, Binding, Mutation};
use d::s_preflight::ReservationFacts;
use rusqlite::Transaction;

pub(super) fn find_request(db: &Connection, request: Id) -> Result<Option<(CommitView, Digest)>> {
    Ok(db.query_row("SELECT id,audit_seq,grant_id,reservation,launch,activation,fingerprint FROM s_requests WHERE id=?",[request],|r|Ok((CommitView {request:r.get(0)?,sequence:r.get(1)?,grant:r.get(2)?,reservation:r.get(3)?,launch:r.get(4)?,activation:r.get(5)?},r.get(6)?))).optional()?)
}

struct Pending {
    kind: &'static str,
    grant: Option<Id>,
    reservation: Option<Id>,
    launch: Option<Id>,
    audit: i64,
    slot: Option<Slot>,
}

impl Runtime {
    pub(super) fn execute(
        &mut self,
        db: &mut Connection,
        epoch: Epoch,
        rejected: &HashSet<Id>,
        request: SRequest,
    ) -> Result<SOutcome> {
        self.reap_abandoned_slots();
        self.check_marker()?;
        if request.request.0 == [0; 16] || request.activation != self.activation.core {
            return Err(Error::Epoch);
        }
        let fingerprint = request_fingerprint(&request)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if db::find_receipt(&tx, request.request)?.is_some() {
            return Err(Error::Conflict);
        }
        if let Some((view, prior)) = find_request(&tx, request.request)? {
            return if prior == fingerprint {
                Ok(SOutcome::StatusOnly(view))
            } else {
                Err(Error::Conflict)
            };
        }
        if rejected.contains(&request.request) {
            return Err(Error::Replay);
        }
        if db::epoch(&tx)? != (epoch, false) || read_activation(&tx)? != self.activation {
            return Err(Error::Epoch);
        }
        db::integrity(&tx)?;
        self.verify_audits(&tx)?;
        let now = self.now()?;
        // Closing the slots precedes control writes. Rollback cannot reopen them.
        self.close_affected_slots(&tx, &request.command)?;
        let result = self.mutate(&tx, epoch, request.request, request.command, now);
        let pending = match result {
            Ok(pending) => pending,
            Err(error) => {
                tx.rollback().map_err(|_| Error::UnknownCommit)?;
                return Err(error);
            }
        };
        let before_commit = (|| {
            tx.execute(
                "INSERT INTO s_requests VALUES(?,?,?,?,?,?,?,?)",
                params![
                    request.request,
                    fingerprint,
                    pending.audit,
                    pending.grant,
                    pending.kind,
                    self.activation.sequence,
                    pending.reservation,
                    pending.launch
                ],
            )?;
            self.reach(FaultPoint::Receipt, &tx)?;
            db::domain_integrity(&tx)?;
            domain_integrity(&tx)?;
            self.verify_audits(&tx)?;
            self.reach(FaultPoint::BeforeCommit, &tx)?;
            let now = self.now()?;
            if pending.slot.as_ref().is_some_and(|slot| {
                now.utc >= slot.deadline.utc || now.monotonic_ms >= slot.deadline.monotonic_ms
            }) {
                return Err(Error::Transition);
            }
            self.check_marker()?;
            self.reach(FaultPoint::CommitUnknown, &tx)?;
            Ok(())
        })();
        if let Err(error) = before_commit {
            tx.rollback().map_err(|_| Error::UnknownCommit)?;
            return Err(error);
        }
        tx.commit().map_err(|_| Error::UnknownCommit)?;
        self.reach(FaultPoint::AfterCommit, db)
            .map_err(|_| Error::UnknownCommit)?;
        let slot_key = pending.slot.map(|slot| {
            let owner = Arc::new(());
            self.slots
                .insert(request.request, (Arc::downgrade(&owner), slot));
            owner
        });
        Ok(SOutcome::Fresh(DurableCommitReceipt {
            view: CommitView {
                request: request.request,
                sequence: pending.audit,
                grant: pending.grant,
                reservation: pending.reservation,
                launch: pending.launch,
                activation: self.activation.sequence,
            },
            slot: slot_key,
        }))
    }

    fn close_affected_slots(&mut self, tx: &Connection, command: &SCommand) -> Result<()> {
        if self.slots.is_empty() {
            return Ok(());
        }
        let (sql, reference) = match command {
            SCommand::RevokeApproval { approval } => (
                "SELECT grant_id FROM s_approval_versions WHERE id=?",
                *approval,
            ),
            SCommand::RevokeRule { rule } => (
                "SELECT grant_id FROM s_approval_versions WHERE rule_id=?",
                *rule,
            ),
            SCommand::InvalidatePrepared { prepared } => {
                ("SELECT grant_id FROM s_grants WHERE prepared=?", *prepared)
            }
            SCommand::Cancel { grant } => {
                ("SELECT grant_id FROM s_grants WHERE grant_id=?", *grant)
            }
            _ => return Ok(()),
        };
        let affected = tx
            .prepare(sql)?
            .query_map([id(reference)], |row| row.get::<_, Id>(0))?
            .collect::<rusqlite::Result<HashSet<_>>>()?;
        self.slots
            .retain(|_, (_, slot)| !affected.contains(&slot.grant));
        Ok(())
    }

    fn mutate(
        &mut self,
        tx: &Transaction<'_>,
        epoch: Epoch,
        request: Id,
        command: SCommand,
        now: d::ClockSample,
    ) -> Result<Pending> {
        let (kind, grant, reservation, launch, g0_seq, slot) = match command {
            SCommand::Policy { candidate } => {
                self.verify_context(&candidate, epoch, now)?;
                tx.execute(
                    "INSERT INTO s_policy_inputs VALUES(?,?,?)",
                    (
                        id(candidate.input().decision.id),
                        Bytes(candidate.fingerprint()),
                        self.activation.sequence,
                    ),
                )?;
                ("policy", None, None, None, None, None)
            }
            SCommand::Rule {
                candidate,
                approval,
            } => {
                self.verify_context(&candidate, epoch, now)?;
                let verified = typed(self.verifier.verify_reserve(&candidate, now))?;
                let scope = Bytes(typed(verified.scope_digest())?);
                if now.utc.unix_millis() != approval.approved_utc {
                    return Err(Error::Clock);
                }
                let reference = approval.reference;
                let prior: Option<(i64, u32, bool)> = tx
                    .query_row(
                        "SELECT version,used,revoked FROM s_rules WHERE id=?",
                        [id(reference.id)],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                    )
                    .optional()?;
                match prior {
                    Some((version, used, revoked)) => {
                        if revoked
                            || integer(reference.version)? <= version
                            || used > approval.max_operations
                        {
                            return Err(Error::Transition);
                        }
                        let prior_scope: Digest = tx.query_row(
                            "SELECT scope FROM s_rule_versions WHERE id=? AND version=?",
                            (id(reference.id), version),
                            |r| r.get(0),
                        )?;
                        if prior_scope != scope {
                            return Err(Error::Binding);
                        }
                        tx.execute(
                            "UPDATE s_rules SET version=? WHERE id=?",
                            (integer(reference.version)?, id(reference.id)),
                        )?;
                    }
                    None => {
                        tx.execute(
                            "INSERT INTO s_rules VALUES(?,?,0,0)",
                            (id(reference.id), integer(reference.version)?),
                        )?;
                    }
                }
                tx.execute(
                    "INSERT INTO s_rule_versions VALUES(?,?,?,?,?,?,?,?)",
                    params![
                        id(reference.id),
                        integer(reference.version)?,
                        approval.event,
                        scope,
                        integer(approval.approved_utc)?,
                        integer(approval.expires_utc)?,
                        approval.max_operations,
                        self.activation.sequence
                    ],
                )?;
                ("rule", None, None, None, None, None)
            }
            SCommand::Register {
                candidate,
                approval,
            } => {
                self.verify_context(&candidate, epoch, now)?;
                let input = candidate.input();
                if approval.fingerprint != Bytes(typed(candidate.request_fingerprint())?)
                    || approval.deadline.utc <= now.utc
                    || approval.deadline.monotonic_ms <= now.monotonic_ms
                {
                    return Err(Error::Binding);
                }
                let registered:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM s_policy_inputs WHERE decision=? AND binding=? AND activation=?)",(id(input.decision.id),Bytes(candidate.fingerprint()),self.activation.sequence),|r|r.get(0))?;
                if !registered {
                    return Err(Error::Ineligible);
                }
                let verified = typed(self.verifier.verify_reserve(&candidate, now))?;
                let scope = Bytes(typed(verified.scope_digest())?);
                self.check_rule(tx, input.parent, scope, now, false)?;
                let g = id(input.grant);
                let changed_mode: bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM s_approval_versions WHERE grant_id=? AND rule_id IS NOT ?)",(g,input.parent.map(|r|id(r.id))),|r|r.get(0))?;
                if changed_mode {
                    return Err(Error::Binding);
                }
                if owns_grant(tx, g)? {
                    let prior: (Digest, Id) = tx.query_row(
                        "SELECT binding,decision FROM s_grants WHERE grant_id=?",
                        [g],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )?;
                    let state: State =
                        tx.query_row("SELECT state FROM grants WHERE id=?", [g], |r| r.get(0))?;
                    if prior != (Bytes(candidate.fingerprint()), id(input.decision.id))
                        || state != State::Pending
                    {
                        return Err(Error::Binding);
                    }
                } else {
                    let binding = Binding {
                        grant: g,
                        decision: id(input.decision.id),
                        input: Bytes(candidate.fingerprint()),
                        epoch,
                    };
                    // The original executor supplies G0 identity and transition checks.
                    let (kind, previous, state) = super::super::mutate(
                        tx,
                        Mutation {
                            request,
                            binding,
                            action: Action::Create { eligible: true },
                        },
                        &mut super::super::Time {
                            clock: Box::new(super::super::ProcessClock(Instant::now())),
                            wall_high: 0,
                            mono_high: 0,
                        },
                    )?;
                    super::super::insert_audit(tx, g, kind, previous, state, None)?;
                    let p = input.preparation;
                    tx.execute(
                        "INSERT INTO s_prepared VALUES(?,?,?,?,?,?,?,?)",
                        params![
                            id(p.id),
                            integer(p.generation)?,
                            id(p.peer.connection),
                            id(p.peer.broker_instance),
                            id(p.peer.worker_instance),
                            integer(p.deadline.utc.unix_millis())?,
                            integer(p.deadline.monotonic_ms)?,
                            self.activation.sequence
                        ],
                    )?;
                    tx.execute(
                        "INSERT INTO s_grants VALUES(?,?,?,?,?,?,?,?)",
                        params![
                            g,
                            id(input.decision.id),
                            id(p.id),
                            id(verified.invocation()),
                            Bytes(candidate.fingerprint()),
                            verified.semantic(),
                            scope,
                            self.activation.sequence
                        ],
                    )?;
                    super::super::transition(tx, g, State::Created, State::Pending, None)?;
                }
                let prior: Option<(u32, bool)> = tx
                    .query_row(
                        "SELECT version,revoked FROM s_approval_heads WHERE id=?",
                        [id(input.approval)],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()?;
                if let Some((version, revoked)) = prior {
                    let old:(Id,Option<Id>,Option<i64>)=tx.query_row("SELECT grant_id,rule_id,rule_version FROM s_approval_versions WHERE id=? AND version=?",(id(input.approval),version),|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
                    if revoked
                        || input.approval_version <= version
                        || old
                            != (
                                g,
                                input.parent.map(|r| id(r.id)),
                                input.parent.map(|r| integer(r.version)).transpose()?,
                            )
                    {
                        return Err(Error::Binding);
                    }
                    tx.execute(
                        "UPDATE s_approval_heads SET version=? WHERE id=?",
                        (input.approval_version, id(input.approval)),
                    )?;
                } else {
                    tx.execute(
                        "INSERT INTO s_approval_heads VALUES(?,?,0)",
                        (id(input.approval), input.approval_version),
                    )?;
                }
                tx.execute(
                    "INSERT INTO s_approval_versions VALUES(?,?,?,?,?,?,?,?,?,?,?)",
                    params![
                        id(input.approval),
                        input.approval_version,
                        g,
                        approval.event,
                        approval.actor,
                        approval.challenge,
                        Bytes(candidate.fingerprint()),
                        input.parent.map(|r| id(r.id)),
                        input.parent.map(|r| integer(r.version)).transpose()?,
                        integer(approval.deadline.utc.unix_millis())?,
                        integer(approval.deadline.monotonic_ms)?
                    ],
                )?;
                let g0 = if tx.query_row::<bool, _, _>(
                    "SELECT NOT EXISTS(SELECT 1 FROM audit WHERE grant_id=? AND state='pending')",
                    [g],
                    |r| r.get(0),
                )? {
                    Some(
                        super::super::insert_audit(
                            tx,
                            g,
                            AuditKind::Transition,
                            Some(State::Created),
                            State::Pending,
                            None,
                        )?
                        .sequence,
                    )
                } else {
                    None
                };
                ("register", Some(g), None, None, g0, None)
            }
            SCommand::Reserve {
                candidate,
                reservation,
            } => {
                self.verify_context(&candidate, epoch, now)?;
                self.check_candidate(tx, &candidate, now, State::Pending, true)?;
                if reservation.is_nil()
                    || tx.query_row::<bool, _, _>(
                        "SELECT EXISTS(SELECT 1 FROM s_reservations WHERE id=?)",
                        [id(reservation)],
                        |r| r.get(0),
                    )?
                {
                    return Err(Error::Conflict);
                }
                let mut nonce = None;
                // Owner approval gives one retry after the first collision, two attempts total.
                for _ in 0..2 {
                    let raw = self.nonces.next()?;
                    if !tx.query_row::<bool, _, _>(
                        "SELECT EXISTS(SELECT 1 FROM nonces WHERE hash=?)",
                        [crate::nonce_hash(&raw)],
                        |r| r.get(0),
                    )? {
                        nonce = Some(raw);
                        break;
                    }
                }
                let raw = nonce.ok_or(Error::Fenced)?;
                let verified = typed(self.verifier.verify_reserve(&candidate, now))?;
                let signed =
                    typed(
                        self.verifier
                            .sign_reservation(verified, reservation, raw, now),
                    )?;
                let input = candidate.input();
                self.check_deadline(tx, input, signed.deadline)?;
                let g = id(input.grant);
                let seq = next_audit(tx)?;
                tx.execute(
                    "INSERT INTO s_approval_claims VALUES(?,?,?,?,?)",
                    (
                        id(input.approval),
                        input.approval_version,
                        id(reservation),
                        g,
                        seq,
                    ),
                )?;
                self.reach(FaultPoint::Claim, tx)?;
                if let Some(reference) = input.parent
                    && tx.execute("UPDATE s_rules SET used=used+1 WHERE id=? AND version=? AND used<3 AND revoked=0",(id(reference.id),integer(reference.version)?))?!=1 {
                    return Err(Error::Ineligible);
                }
                self.reach(FaultPoint::Quota, tx)?;
                let wall = integer(signed.deadline.utc.unix_millis())?;
                let mono = integer(signed.deadline.monotonic_ms)?;
                tx.execute("INSERT INTO g0_reservations(grant_id,decision_id,operation_digest,boot,policy,grant_epoch,nonce_hash,wall_deadline,mono_deadline) VALUES(?,?,?,?,?,?,?,?,?)",params![g,id(input.decision.id),Bytes(candidate.fingerprint()),epoch.boot,epoch.policy,epoch.grant,Bytes(signed.nonce_hash),wall,mono])?;
                self.reach(FaultPoint::G0Reservation, tx)?;
                tx.execute(
                    "INSERT INTO nonces VALUES(?,?)",
                    (Bytes(signed.nonce_hash), g),
                )?;
                self.reach(FaultPoint::Nonce, tx)?;
                tx.execute(
                    "INSERT INTO s_reservations VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
                    params![
                        id(reservation),
                        g,
                        id(input.approval),
                        Bytes(signed.nonce_hash),
                        signed.operation_mac,
                        signed.permit_mac,
                        Bytes(signed.wire_digest),
                        integer(now.utc.unix_millis())?,
                        integer(now.monotonic_ms)?,
                        wall,
                        mono,
                        input.ttl_ms,
                        self.activation.sequence,
                        seq
                    ],
                )?;
                self.reach(FaultPoint::Reservation, tx)?;
                super::super::transition(
                    tx,
                    g,
                    State::Pending,
                    State::Reserved,
                    Some((wall, mono)),
                )?;
                self.reach(FaultPoint::State, tx)?;
                let g0 = super::super::insert_audit(
                    tx,
                    g,
                    AuditKind::Transition,
                    Some(State::Pending),
                    State::Reserved,
                    None,
                )?;
                let deadline = signed.deadline;
                let slot = Slot {
                    grant: g,
                    peer: input.preparation.peer,
                    deadline,
                    response: SResponse::Permit(signed.into_permit()),
                };
                (
                    "reserve",
                    Some(g),
                    Some(id(reservation)),
                    None,
                    Some(g0.sequence),
                    Some(slot),
                )
            }
            SCommand::Consume { candidate, launch } => {
                let context = candidate.context();
                self.verify_context(context, epoch, now)?;
                self.check_candidate(tx, context, now, State::Reserved, false)?;
                let g = id(context.input().grant);
                let reservation = id(typed(candidate.reservation())?);
                let (nonce,wall,mono,operation_mac,permit_mac):(Digest,i64,i64,String,String)=tx.query_row("SELECT nonce_hash,wall,mono,operation_mac,permit_mac FROM s_reservations WHERE id=? AND grant_id=? AND activation=?",(reservation,g,self.activation.sequence),|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
                let deadline = d::Deadline {
                    utc: typed(d::UtcMillis::from_unix_millis(wall as u64))?,
                    monotonic_ms: mono as u64,
                };
                typed(self.verifier.verify_consume(
                    &candidate,
                    ReservationFacts {
                        id: uuid(reservation),
                        nonce_hash: nonce.0,
                        deadline,
                        operation_mac: &operation_mac,
                        permit_mac: &permit_mac,
                    },
                    now,
                ))?;
                self.check_deadline(tx, context.input(), deadline)?;
                if launch.is_nil() {
                    return Err(Error::Input);
                }
                tx.execute(
                    "INSERT INTO g0_execution_bindings(grant_id,operation_digest) VALUES(?,?)",
                    (g, Bytes(context.fingerprint())),
                )?;
                tx.execute("INSERT INTO execution VALUES(?,'unknown',0)", [g])?;
                self.reach(FaultPoint::Execution, tx)?;
                let seq = next_audit(tx)?;
                let response_digest = digest(
                    &[
                        &g.0[..],
                        &reservation.0[..],
                        launch.as_bytes(),
                        self.activation.core.0.as_slice(),
                    ]
                    .concat(),
                );
                tx.execute(
                    "INSERT INTO s_launch_once VALUES(?,?,?,?,?)",
                    (id(launch), reservation, g, response_digest, seq),
                )?;
                self.reach(FaultPoint::Launch, tx)?;
                tx.execute("INSERT INTO s_execution_facts(grant_id,kind,audit_seq) VALUES(?,'launch_registered',?)",(g,seq))?;
                self.reach(FaultPoint::Facts, tx)?;
                super::super::transition(tx, g, State::Reserved, State::Consumed, None)?;
                self.reach(FaultPoint::State, tx)?;
                let g0 = super::super::insert_audit(
                    tx,
                    g,
                    AuditKind::Transition,
                    Some(State::Reserved),
                    State::Consumed,
                    None,
                )?;
                let p = context.input().preparation;
                let slot = Slot {
                    grant: g,
                    peer: p.peer,
                    deadline,
                    response: SResponse::Launch(LaunchAuthorization {
                        metadata: LaunchMetadata {
                            launch: id(launch),
                            reservation,
                            grant: g,
                            peer: p.peer,
                            prepared: p.id,
                            prepared_generation: p.generation,
                            activation: self.activation,
                            deadline,
                        },
                    }),
                };
                (
                    "consume",
                    Some(g),
                    Some(reservation),
                    Some(id(launch)),
                    Some(g0.sequence),
                    Some(slot),
                )
            }
            SCommand::RevokeApproval { approval } => {
                if tx.execute(
                    "UPDATE s_approval_heads SET revoked=1 WHERE id=? AND revoked=0",
                    [id(approval)],
                )? != 1
                {
                    return Err(Error::Transition);
                }
                ("revoke_approval", None, None, None, None, None)
            }
            SCommand::RevokeRule { rule } => {
                if tx.execute(
                    "UPDATE s_rules SET revoked=1 WHERE id=? AND revoked=0",
                    [id(rule)],
                )? != 1
                {
                    return Err(Error::Transition);
                }
                ("revoke_rule", None, None, None, None, None)
            }
            SCommand::InvalidatePrepared { prepared } => {
                let g: Id = tx.query_row(
                    "SELECT grant_id FROM s_grants WHERE prepared=?",
                    [id(prepared)],
                    |r| r.get(0),
                )?;
                tx.execute(
                    "INSERT INTO s_invalidated_prepared VALUES(?,?)",
                    (id(prepared), next_audit(tx)?),
                )?;
                let previous: State =
                    tx.query_row("SELECT state FROM grants WHERE id=?", [g], |r| r.get(0))?;
                super::super::transition(tx, g, previous, State::Invalidated, None)?;
                let g0 = super::super::insert_audit(
                    tx,
                    g,
                    AuditKind::Transition,
                    Some(previous),
                    State::Invalidated,
                    None,
                )?;
                ("invalidate", Some(g), None, None, Some(g0.sequence), None)
            }
            SCommand::Cancel { grant } => {
                let g = id(grant);
                if !owns_grant(tx, g)? {
                    return Err(Error::Scope);
                }
                let previous: State =
                    tx.query_row("SELECT state FROM grants WHERE id=?", [g], |r| r.get(0))?;
                let g0 = if previous == State::Consumed {
                    if tx.execute("UPDATE execution SET cancel_requested=1 WHERE grant_id=? AND cancel_requested=0",[g])?!=1 {return Err(Error::Transition);}
                    tx.execute("INSERT INTO s_execution_facts(grant_id,kind,audit_seq) VALUES(?,'cancel_requested',?)",(g,next_audit(tx)?))?;
                    super::super::insert_audit(
                        tx,
                        g,
                        AuditKind::CancelRequested,
                        None,
                        State::Consumed,
                        None,
                    )?
                } else {
                    super::super::transition(tx, g, previous, State::Revoked, None)?;
                    super::super::insert_audit(
                        tx,
                        g,
                        AuditKind::Transition,
                        Some(previous),
                        State::Revoked,
                        None,
                    )?
                };
                ("cancel", Some(g), None, None, Some(g0.sequence), None)
            }
        };
        self.reach(FaultPoint::G0Audit, tx)?;
        let audit = self.audit(tx, kind, grant, g0_seq, Some(request))?;
        self.reach(FaultPoint::Audit, tx)?;
        Ok(Pending {
            kind,
            grant,
            reservation,
            launch,
            audit,
            slot,
        })
    }

    fn verify_context(
        &self,
        candidate: &PrecommitCandidate,
        epoch: Epoch,
        now: d::ClockSample,
    ) -> Result<()> {
        let e = candidate.input().epochs;
        if e.core.0 != uuid(epoch.boot)
            || e.policy.0 != u64::from(epoch.policy)
            || e.grant.0 != u64::from(epoch.grant)
        {
            return Err(Error::Epoch);
        }
        typed(self.verifier.verify_reserve(candidate, now))?;
        Ok(())
    }
    fn check_rule(
        &self,
        tx: &Connection,
        parent: Option<d::RuleRef>,
        scope: Digest,
        now: d::ClockSample,
        budget: bool,
    ) -> Result<()> {
        if let Some(reference) = parent {
            let (version,used,revoked,actual,from,end,activation):(i64,u32,bool,Digest,i64,i64,i64)=tx.query_row("SELECT r.version,r.used,r.revoked,v.scope,v.approved_utc,v.expires_utc,v.activation FROM s_rules r JOIN s_rule_versions v ON v.id=r.id AND v.version=r.version WHERE r.id=?",[id(reference.id)],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?)))?;
            if version != integer(reference.version)?
                || revoked
                || activation != self.activation.sequence
                || scope != actual
                || integer(now.utc.unix_millis())? < from
                || integer(now.utc.unix_millis())? >= end
                || (budget && used >= 3)
            {
                return Err(Error::Ineligible);
            }
        }
        Ok(())
    }
    fn check_candidate(
        &self,
        tx: &Connection,
        candidate: &PrecommitCandidate,
        now: d::ClockSample,
        state: State,
        budget: bool,
    ) -> Result<()> {
        let i = candidate.input();
        let (binding,scope,activation,current):(Digest,Digest,i64,State)=tx.query_row("SELECT s.binding,s.scope,s.activation,g.state FROM s_grants s JOIN grants g ON g.id=s.grant_id WHERE s.grant_id=?",[id(i.grant)],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
        if binding != Bytes(candidate.fingerprint())
            || activation != self.activation.sequence
            || current != state
        {
            return Err(Error::Binding);
        }
        let (version,revoked,a_grant,a_binding,parent,parent_version,wall,mono):(u32,bool,Id,Digest,Option<Id>,Option<i64>,i64,i64)=tx.query_row("SELECT h.version,h.revoked,a.grant_id,a.binding,a.rule_id,a.rule_version,a.wall,a.mono FROM s_approval_heads h JOIN s_approval_versions a ON a.id=h.id AND a.version=h.version WHERE h.id=?",[id(i.approval)],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?)))?;
        if version != i.approval_version
            || revoked
            || a_grant != id(i.grant)
            || a_binding != binding
            || parent != i.parent.map(|r| id(r.id))
            || parent_version != i.parent.map(|r| integer(r.version)).transpose()?
            || integer(now.utc.unix_millis())? >= wall
            || integer(now.monotonic_ms)? >= mono
        {
            return Err(Error::Binding);
        }
        if tx.query_row::<bool, _, _>(
            "SELECT EXISTS(SELECT 1 FROM s_invalidated_prepared WHERE id=?)",
            [id(i.preparation.id)],
            |r| r.get(0),
        )? {
            return Err(Error::Binding);
        }
        self.check_rule(tx, i.parent, scope, now, budget)
    }
    fn check_deadline(
        &self,
        tx: &Connection,
        i: &d::s_preflight::ReserveInput,
        deadline: d::Deadline,
    ) -> Result<()> {
        let (wall, mono): (i64, i64) = tx.query_row(
            "SELECT wall,mono FROM s_approval_versions WHERE id=? AND version=?",
            (id(i.approval), i.approval_version),
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if integer(deadline.utc.unix_millis())? > wall
            || integer(deadline.monotonic_ms)? > mono
            || deadline.monotonic_ms > i64::MAX as u64
        {
            return Err(Error::Clock);
        }
        if let Some(r) = i.parent {
            let end: i64 = tx.query_row(
                "SELECT expires_utc FROM s_rule_versions WHERE id=? AND version=?",
                (id(r.id), integer(r.version)?),
                |r| r.get(0),
            )?;
            if integer(deadline.utc.unix_millis())? > end {
                return Err(Error::Clock);
            }
        }
        Ok(())
    }
    pub(super) fn check_current_grant(
        &self,
        tx: &Connection,
        grant: Id,
        now: d::ClockSample,
    ) -> Result<()> {
        let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM s_grants s JOIN grants g ON g.id=s.grant_id JOIN s_approval_versions a ON a.grant_id=g.id JOIN s_approval_heads h ON h.id=a.id AND h.version=a.version JOIN s_reservations r ON r.grant_id=g.id AND r.approval=a.id JOIN s_approval_claims c ON c.approval=a.id AND c.version=a.version WHERE g.id=? AND g.state IN ('reserved','consumed') AND h.revoked=0 AND s.activation=? AND a.wall>? AND a.mono>? AND NOT EXISTS(SELECT 1 FROM s_invalidated_prepared p WHERE p.id=s.prepared) AND (a.rule_id IS NULL OR EXISTS(SELECT 1 FROM s_rules u JOIN s_rule_versions v ON v.id=u.id AND v.version=u.version WHERE u.id=a.rule_id AND u.version=a.rule_version AND u.revoked=0 AND v.activation=? AND v.expires_utc>?)))",params![grant,self.activation.sequence,integer(now.utc.unix_millis())?,integer(now.monotonic_ms)?,self.activation.sequence,integer(now.utc.unix_millis())?],|r|r.get(0))?;
        if !active {
            return Err(Error::Fenced);
        }
        Ok(())
    }
    pub(super) fn audit(
        &self,
        tx: &Connection,
        kind: &str,
        grant: Option<Id>,
        g0: Option<i64>,
        request: Option<Id>,
    ) -> Result<i64> {
        let sequence = next_audit(tx)?;
        let previous: Option<String> = tx
            .query_row(
                "SELECT mac FROM s_audit WHERE activation=? ORDER BY seq DESC LIMIT 1",
                [self.activation.sequence],
                |r| r.get(0),
            )
            .optional()?;
        let event = audit_event(self.activation.sequence, kind, grant, g0, request);
        let encoded = serde_json::to_vec(&event).map_err(|_| Error::Input)?;
        let mac = typed(self.verifier.audit_mac(&d::AuditLink {
            segment_id: uuid(self.activation.core),
            sequence: sequence as u64,
            previous_mac: previous.as_deref(),
            event_digest: typed(d::redacted_event_digest(&encoded))?,
            key_id: self.verifier.key_id(),
            key_generation: self.activation.generation,
        }))?;
        // Event digest uses the frozen typed encoding through I06-D.
        let event_digest = event_digest(&encoded)?;
        tx.execute(
            "INSERT INTO s_audit VALUES(?,?,?,?,?,?,?,?,?)",
            params![
                sequence,
                self.activation.sequence,
                kind,
                grant,
                g0,
                request,
                event_digest,
                previous,
                mac
            ],
        )?;
        Ok(sequence)
    }
    pub(super) fn verify_audits(&self, db: &Connection) -> Result<()> {
        verify_structure(db)?;
        let rows = audit_rows(db)?;
        let mut prior: HashMap<i64, String> = HashMap::new();
        for row in rows {
            let (seq, activation, kind, grant, g0, request, stored, previous, mac) = row;
            let encoded = serde_json::to_vec(&audit_event(activation, &kind, grant, g0, request))
                .map_err(|_| Error::Corrupt)?;
            if stored != event_digest(&encoded)? || previous.as_ref() != prior.get(&activation) {
                return Err(Error::Corrupt);
            }
            if activation == self.activation.sequence {
                self.verifier
                    .verify_audit(
                        &d::AuditLink {
                            segment_id: uuid(self.activation.core),
                            sequence: seq as u64,
                            previous_mac: previous.as_deref(),
                            event_digest: typed(d::redacted_event_digest(&encoded))?,
                            key_id: self.verifier.key_id(),
                            key_generation: self.activation.generation,
                        },
                        &mac,
                    )
                    .map_err(|_| Error::Corrupt)?;
            }
            prior.insert(activation, mac);
        }
        Ok(())
    }
}

pub(super) fn verify_structure(db: &Connection) -> Result<()> {
    let mut previous: HashMap<i64, String> = HashMap::new();
    let mut last_activation = 0;
    for (index, row) in audit_rows(db)?.into_iter().enumerate() {
        let (seq, activation, kind, grant, g0, request, stored, prior, mac) = row;
        let encoded = serde_json::to_vec(&audit_event(activation, &kind, grant, g0, request))
            .map_err(|_| Error::Corrupt)?;
        if seq != index as i64 + 1
            || activation < last_activation
            || stored != event_digest(&encoded)?
            || prior.as_ref() != previous.get(&activation)
            || mac.len() != 64
            || !mac.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(Error::Corrupt);
        }
        last_activation = activation;
        previous.insert(activation, mac);
    }
    Ok(())
}

fn next_audit(tx: &Connection) -> Result<i64> {
    Ok(
        tx.query_row("SELECT coalesce(max(seq),0)+1 FROM s_audit", [], |r| {
            r.get(0)
        })?,
    )
}
fn audit_event(
    activation: i64,
    kind: &str,
    grant: Option<Id>,
    g0: Option<i64>,
    request: Option<Id>,
) -> serde_json::Value {
    json!({"activation":activation,"kind":kind,"grant":grant.map(|v|uuid(v).to_string()),"g0":g0,"request":request.map(|v|uuid(v).to_string())})
}
fn event_digest(encoded: &[u8]) -> Result<Digest> {
    let value: serde_json::Value = serde_json::from_slice(encoded).map_err(|_| Error::Corrupt)?;
    // Identical typed frame hash to redacted_event_digest, without exposing its MAC API.
    let bytes = agentguard_protocol::crypto::typed_encode(&value).map_err(|_| Error::Corrupt)?;
    Ok(digest(&bytes))
}
type AuditRow = (
    i64,
    i64,
    String,
    Option<Id>,
    Option<i64>,
    Option<Id>,
    Digest,
    Option<String>,
    String,
);
fn audit_rows(db: &Connection) -> Result<Vec<AuditRow>> {
    Ok(db.prepare("SELECT seq,activation,kind,grant_id,g0_seq,request,event_digest,previous_mac,mac FROM s_audit ORDER BY seq")?.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?)))?.collect::<rusqlite::Result<_>>()?)
}
fn request_fingerprint(request: &SRequest) -> Result<Digest> {
    let value = match &request.command {
        SCommand::Policy { candidate } => {
            json!(["policy", typed(candidate.request_fingerprint())?])
        }
        SCommand::Register {
            candidate,
            approval,
        } => json!([
            "register",
            typed(candidate.request_fingerprint())?,
            approval.actor.0,
            approval.event.0,
            approval.challenge.0,
            integer(approval.deadline.utc.unix_millis())?,
            approval.deadline.monotonic_ms
        ]),
        SCommand::Rule {
            candidate,
            approval,
        } => json!([
            "rule",
            typed(candidate.request_fingerprint())?,
            approval.reference.id.to_string(),
            approval.reference.version,
            approval.event.0,
            integer(approval.approved_utc)?,
            integer(approval.expires_utc)?,
            approval.max_operations
        ]),
        SCommand::Reserve {
            candidate,
            reservation,
        } => json!([
            "reserve",
            typed(candidate.request_fingerprint())?,
            reservation.to_string()
        ]),
        SCommand::Consume { candidate, launch } => json!([
            "consume",
            candidate.fingerprint(),
            typed(candidate.context().request_fingerprint())?,
            launch.to_string()
        ]),
        SCommand::RevokeApproval { approval } => json!(["revoke_approval", approval.to_string()]),
        SCommand::RevokeRule { rule } => json!(["revoke_rule", rule.to_string()]),
        SCommand::InvalidatePrepared { prepared } => json!(["invalidate", prepared.to_string()]),
        SCommand::Cancel { grant } => json!(["cancel", grant.to_string()]),
    };
    Ok(digest(
        &serde_json::to_vec(&(request.activation.0, value)).map_err(|_| Error::Input)?,
    ))
}
