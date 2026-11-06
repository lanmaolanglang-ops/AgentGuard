-- The caller verifies the old schema and invalidates unfinished legacy grants first.
CREATE TABLE g0_cutover (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    audit_seq INTEGER NOT NULL CHECK(audit_seq >= 0)
) STRICT;
INSERT INTO g0_cutover SELECT 1, coalesce(max(seq),0) FROM audit;

CREATE TABLE g0_heads (
    grant_id BLOB PRIMARY KEY REFERENCES grants(id),
    state TEXT NOT NULL,
    kind TEXT GENERATED ALWAYS AS ('transition') STORED,
    FOREIGN KEY(grant_id,kind,state) REFERENCES audit(grant_id,kind,state)
        DEFERRABLE INITIALLY DEFERRED
) STRICT;
INSERT INTO g0_heads(grant_id,state) SELECT id,state FROM grants;

-- operation_digest is the I05 synthetic input binding. No wire digest is generated.
CREATE TABLE g0_reservations (
    grant_id BLOB PRIMARY KEY REFERENCES grants(id),
    decision_id BLOB NOT NULL REFERENCES decisions(id),
    operation_digest BLOB NOT NULL CHECK(length(operation_digest) = 32),
    boot BLOB NOT NULL CHECK(length(boot) = 16),
    policy INTEGER NOT NULL CHECK(policy >= 0),
    grant_epoch INTEGER NOT NULL CHECK(grant_epoch >= 0),
    nonce_hash BLOB NOT NULL REFERENCES nonces(hash) DEFERRABLE INITIALLY DEFERRED,
    wall_deadline INTEGER NOT NULL CHECK(wall_deadline >= 0),
    mono_deadline INTEGER NOT NULL CHECK(mono_deadline >= 0),
    kind TEXT GENERATED ALWAYS AS ('transition') STORED,
    state TEXT GENERATED ALWAYS AS ('reserved') STORED,
    UNIQUE(grant_id,operation_digest),
    FOREIGN KEY(grant_id,kind,state) REFERENCES audit(grant_id,kind,state)
        DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE TABLE g0_execution_bindings (
    grant_id BLOB PRIMARY KEY REFERENCES execution(grant_id) DEFERRABLE INITIALLY DEFERRED,
    operation_digest BLOB NOT NULL CHECK(length(operation_digest) = 32),
    kind TEXT GENERATED ALWAYS AS ('transition') STORED,
    state TEXT GENERATED ALWAYS AS ('consumed') STORED,
    cancel_kind TEXT CHECK(cancel_kind='cancel_requested'),
    failure_kind TEXT CHECK(failure_kind='worker_failed'),
    FOREIGN KEY(grant_id,operation_digest) REFERENCES g0_reservations(grant_id,operation_digest),
    FOREIGN KEY(grant_id,kind,state) REFERENCES audit(grant_id,kind,state)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(grant_id,cancel_kind,state) REFERENCES audit(grant_id,kind,state)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(grant_id,failure_kind,state) REFERENCES audit(grant_id,kind,state)
        DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE TRIGGER g0_cutover_no_insert BEFORE INSERT ON g0_cutover
BEGIN SELECT RAISE(ABORT,'G0 immutable cutover'); END;
CREATE TRIGGER g0_cutover_no_update BEFORE UPDATE ON g0_cutover
BEGIN SELECT RAISE(ABORT,'G0 immutable cutover'); END;
CREATE TRIGGER g0_cutover_no_delete BEFORE DELETE ON g0_cutover
BEGIN SELECT RAISE(ABORT,'G0 immutable cutover'); END;

CREATE TRIGGER g0_decision_insert BEFORE INSERT ON decisions
WHEN EXISTS(SELECT 1 FROM decisions WHERE id=NEW.id OR rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'G0 decision conflict'); END;
CREATE TRIGGER g0_grant_insert BEFORE INSERT ON grants
WHEN NEW.state!='created' OR NEW.wall_deadline IS NOT NULL OR NEW.mono_deadline IS NOT NULL
    OR EXISTS(SELECT 1 FROM grants WHERE id=NEW.id OR decision=NEW.decision OR rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'G0 grant insert conflict'); END;
CREATE TRIGGER g0_grant_identity BEFORE UPDATE ON grants
WHEN NEW.id IS NOT OLD.id OR NEW.decision IS NOT OLD.decision OR NEW.rowid IS NOT OLD.rowid
BEGIN SELECT RAISE(ABORT,'G0 immutable grant binding'); END;
CREATE TRIGGER g0_grant_no_delete BEFORE DELETE ON grants
BEGIN SELECT RAISE(ABORT,'G0 immutable grant history'); END;
CREATE TRIGGER g0_prior_audit BEFORE UPDATE OF state ON grants
WHEN OLD.state IS NOT (SELECT state FROM audit WHERE grant_id=OLD.id AND kind='transition'
                       ORDER BY seq DESC LIMIT 1)
BEGIN SELECT RAISE(ABORT,'G0 missing prior audit'); END;

CREATE TRIGGER g0_reservation_insert BEFORE INSERT ON g0_reservations
WHEN EXISTS(SELECT 1 FROM g0_reservations WHERE grant_id=NEW.grant_id OR rowid=NEW.rowid)
    OR NOT EXISTS(
        SELECT 1 FROM grants g JOIN decisions d ON d.id=g.decision JOIN meta m ON m.singleton=1
        WHERE g.id=NEW.grant_id AND g.state='pending'
          AND g.wall_deadline IS NULL AND g.mono_deadline IS NULL AND d.eligible=1
          AND d.id=NEW.decision_id AND d.input_digest=NEW.operation_digest
          AND d.boot=NEW.boot AND d.policy=NEW.policy AND d.grant_epoch=NEW.grant_epoch
          AND m.boot=NEW.boot AND m.policy=NEW.policy AND m.grant_epoch=NEW.grant_epoch
          AND m.archived=0)
BEGIN SELECT RAISE(ABORT,'G0 reservation binding'); END;
CREATE TRIGGER g0_reservation_no_update BEFORE UPDATE ON g0_reservations
BEGIN SELECT RAISE(ABORT,'G0 immutable reservation'); END;
CREATE TRIGGER g0_reservation_no_delete BEFORE DELETE ON g0_reservations
BEGIN SELECT RAISE(ABORT,'G0 immutable reservation'); END;

CREATE TRIGGER g0_nonce_insert BEFORE INSERT ON nonces
WHEN EXISTS(SELECT 1 FROM nonces WHERE hash=NEW.hash OR grant_id=NEW.grant_id OR rowid=NEW.rowid)
    OR NOT EXISTS(SELECT 1 FROM g0_reservations WHERE grant_id=NEW.grant_id AND nonce_hash=NEW.hash)
BEGIN SELECT RAISE(ABORT,'G0 nonce binding'); END;
CREATE TRIGGER g0_deadline_update BEFORE UPDATE ON grants
WHEN (NEW.wall_deadline IS NOT OLD.wall_deadline OR NEW.mono_deadline IS NOT OLD.mono_deadline)
    AND NOT (OLD.state='pending' AND NEW.state='reserved'
      AND OLD.wall_deadline IS NULL AND OLD.mono_deadline IS NULL
      AND NEW.wall_deadline IS NOT NULL AND NEW.mono_deadline IS NOT NULL
      AND EXISTS(
          SELECT 1 FROM g0_reservations r JOIN nonces n ON n.grant_id=r.grant_id AND n.hash=r.nonce_hash
          WHERE r.grant_id=OLD.id AND r.decision_id=OLD.decision
            AND r.wall_deadline=NEW.wall_deadline AND r.mono_deadline=NEW.mono_deadline))
BEGIN SELECT RAISE(ABORT,'G0 immutable deadline'); END;
CREATE TRIGGER g0_reserved_update BEFORE UPDATE OF state ON grants
WHEN NEW.state='reserved' AND NOT EXISTS(
    SELECT 1 FROM g0_reservations r JOIN nonces n ON n.grant_id=r.grant_id AND n.hash=r.nonce_hash
    JOIN meta m ON m.singleton=1
    WHERE r.grant_id=OLD.id AND r.decision_id=OLD.decision
      AND r.wall_deadline=NEW.wall_deadline AND r.mono_deadline=NEW.mono_deadline
      AND r.boot=m.boot AND r.policy=m.policy AND r.grant_epoch=m.grant_epoch AND m.archived=0)
BEGIN SELECT RAISE(ABORT,'G0 reserved companions'); END;

CREATE TRIGGER g0_execution_binding_insert BEFORE INSERT ON g0_execution_bindings
WHEN NEW.cancel_kind IS NOT NULL OR NEW.failure_kind IS NOT NULL
    OR EXISTS(SELECT 1 FROM g0_execution_bindings WHERE grant_id=NEW.grant_id OR rowid=NEW.rowid)
    OR NOT EXISTS(
      SELECT 1 FROM g0_reservations r JOIN grants g ON g.id=r.grant_id
      JOIN nonces n ON n.grant_id=r.grant_id AND n.hash=r.nonce_hash
      JOIN meta m ON m.singleton=1
      WHERE r.grant_id=NEW.grant_id AND r.operation_digest=NEW.operation_digest
        AND g.state='reserved' AND r.boot=m.boot AND r.policy=m.policy
        AND r.grant_epoch=m.grant_epoch AND m.archived=0)
BEGIN SELECT RAISE(ABORT,'G0 execution binding'); END;
CREATE TRIGGER g0_execution_binding_update BEFORE UPDATE ON g0_execution_bindings
WHEN NEW.grant_id IS NOT OLD.grant_id OR NEW.operation_digest IS NOT OLD.operation_digest
    OR NEW.rowid IS NOT OLD.rowid
    OR NOT EXISTS(
        SELECT 1 FROM execution e WHERE e.grant_id=NEW.grant_id
        AND NEW.cancel_kind IS CASE WHEN e.cancel_requested=1 THEN 'cancel_requested' END
        AND NEW.failure_kind IS CASE WHEN e.state='failed' THEN 'worker_failed' END)
BEGIN SELECT RAISE(ABORT,'G0 immutable execution binding'); END;
CREATE TRIGGER g0_execution_binding_no_delete BEFORE DELETE ON g0_execution_bindings
BEGIN SELECT RAISE(ABORT,'G0 immutable execution binding'); END;
CREATE TRIGGER g0_execution_insert BEFORE INSERT ON execution
WHEN NEW.state!='unknown' OR NEW.cancel_requested!=0
    OR EXISTS(SELECT 1 FROM execution WHERE grant_id=NEW.grant_id OR rowid=NEW.rowid)
    OR NOT EXISTS(SELECT 1 FROM g0_execution_bindings WHERE grant_id=NEW.grant_id)
BEGIN SELECT RAISE(ABORT,'G0 execution insert'); END;
CREATE TRIGGER g0_execution_identity BEFORE UPDATE ON execution
WHEN NEW.grant_id IS NOT OLD.grant_id OR NEW.rowid IS NOT OLD.rowid
    OR NEW.cancel_requested<OLD.cancel_requested
    OR (OLD.state='failed' AND NEW.state!='failed')
    OR NOT EXISTS(SELECT 1 FROM g0_execution_bindings WHERE grant_id=OLD.grant_id)
BEGIN SELECT RAISE(ABORT,'G0 immutable execution history'); END;
CREATE TRIGGER g0_execution_proof AFTER UPDATE ON execution
BEGIN
    UPDATE g0_execution_bindings
    SET cancel_kind=CASE WHEN NEW.cancel_requested=1 THEN 'cancel_requested' END,
        failure_kind=CASE WHEN NEW.state='failed' THEN 'worker_failed' END
    WHERE grant_id=NEW.grant_id;
END;
CREATE TRIGGER g0_execution_no_delete BEFORE DELETE ON execution
BEGIN SELECT RAISE(ABORT,'G0 immutable execution history'); END;
CREATE TRIGGER g0_consumed_update BEFORE UPDATE OF state ON grants
WHEN NEW.state='consumed' AND NOT EXISTS(
    SELECT 1 FROM execution e JOIN g0_execution_bindings b ON b.grant_id=e.grant_id
    WHERE e.grant_id=OLD.id AND e.state='unknown' AND e.cancel_requested=0)
BEGIN SELECT RAISE(ABORT,'G0 consumed companions'); END;

CREATE TRIGGER g0_heads_insert BEFORE INSERT ON g0_heads
WHEN EXISTS(SELECT 1 FROM g0_heads WHERE grant_id=NEW.grant_id OR rowid=NEW.rowid)
    OR NEW.state IS NOT (SELECT state FROM grants WHERE id=NEW.grant_id)
BEGIN SELECT RAISE(ABORT,'G0 head insert'); END;
CREATE TRIGGER g0_heads_update BEFORE UPDATE ON g0_heads
WHEN NEW.grant_id IS NOT OLD.grant_id OR NEW.rowid IS NOT OLD.rowid
    OR NEW.state IS NOT (SELECT state FROM grants WHERE id=NEW.grant_id)
BEGIN SELECT RAISE(ABORT,'G0 head update'); END;
CREATE TRIGGER g0_heads_no_delete BEFORE DELETE ON g0_heads
BEGIN SELECT RAISE(ABORT,'G0 immutable head'); END;
CREATE TRIGGER g0_grant_head_insert AFTER INSERT ON grants
BEGIN INSERT INTO g0_heads(grant_id,state) VALUES(NEW.id,NEW.state); END;
CREATE TRIGGER g0_grant_head_update AFTER UPDATE OF state ON grants
BEGIN UPDATE g0_heads SET state=NEW.state WHERE grant_id=NEW.id; END;

CREATE TRIGGER g0_audit_insert BEFORE INSERT ON audit
WHEN EXISTS(SELECT 1 FROM audit WHERE seq=NEW.seq OR request=NEW.request
               OR (grant_id=NEW.grant_id AND kind=NEW.kind AND state=NEW.state))
    OR NEW.state IS NOT (SELECT state FROM grants WHERE id=NEW.grant_id)
    OR (NEW.kind='transition' AND NEW.previous IS NOT (
        SELECT state FROM audit WHERE grant_id=NEW.grant_id AND kind='transition'
        ORDER BY seq DESC LIMIT 1))
    OR (NEW.kind='cancel_requested' AND NOT EXISTS(
        SELECT 1 FROM execution WHERE grant_id=NEW.grant_id AND cancel_requested=1))
    OR (NEW.kind='worker_failed' AND NOT EXISTS(
        SELECT 1 FROM execution WHERE grant_id=NEW.grant_id AND state='failed'))
BEGIN SELECT RAISE(ABORT,'G0 audit binding'); END;
CREATE TRIGGER g0_audit_order AFTER INSERT ON audit
WHEN NEW.seq <= (SELECT audit_seq FROM g0_cutover WHERE id=1)
    OR EXISTS(SELECT 1 FROM audit WHERE seq>NEW.seq)
BEGIN SELECT RAISE(ABORT,'G0 audit sequence'); END;

CREATE VIEW g0_binding_faults AS
SELECT g.id FROM grants g JOIN decisions d ON d.id=g.decision
LEFT JOIN g0_reservations r ON r.grant_id=g.id
LEFT JOIN nonces n ON n.grant_id=g.id
WHERE g.state IS NOT (SELECT state FROM g0_heads WHERE grant_id=g.id)
   OR (r.grant_id IS NOT NULL) != EXISTS(
        SELECT 1 FROM audit WHERE grant_id=g.id AND kind='transition' AND state='reserved'
        AND seq>(SELECT audit_seq FROM g0_cutover WHERE id=1))
   OR (r.grant_id IS NOT NULL AND (
        r.decision_id IS NOT d.id OR r.operation_digest IS NOT d.input_digest
        OR r.boot IS NOT d.boot OR r.policy IS NOT d.policy OR r.grant_epoch IS NOT d.grant_epoch
        OR r.wall_deadline IS NOT g.wall_deadline OR r.mono_deadline IS NOT g.mono_deadline
        OR r.nonce_hash IS NOT n.hash))
   OR EXISTS(SELECT 1 FROM g0_execution_bindings WHERE grant_id=g.id) != EXISTS(
        SELECT 1 FROM audit WHERE grant_id=g.id AND kind='transition' AND state='consumed'
        AND seq>(SELECT audit_seq FROM g0_cutover WHERE id=1))
   OR EXISTS(SELECT 1 FROM g0_execution_bindings b JOIN execution e ON e.grant_id=b.grant_id
        WHERE b.grant_id=g.id AND (
            b.cancel_kind IS NOT CASE WHEN e.cancel_requested=1 THEN 'cancel_requested' END
            OR b.failure_kind IS NOT CASE WHEN e.state='failed' THEN 'worker_failed' END));
