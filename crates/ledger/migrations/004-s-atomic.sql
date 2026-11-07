-- S1 explicit synthetic fixture only. Versions 001-003 remain the v3 path.
CREATE TABLE s_activation_history (
    seq INTEGER PRIMARY KEY CHECK(seq>0),
    lineage BLOB NOT NULL CHECK(length(lineage)=16),
    core BLOB NOT NULL UNIQUE CHECK(length(core)=16),
    clock_domain BLOB NOT NULL UNIQUE CHECK(length(clock_domain)=16),
    os_boot BLOB NOT NULL CHECK(length(os_boot)=16),
    key_id TEXT NOT NULL UNIQUE CHECK(length(key_id) BETWEEN 1 AND 64),
    generation INTEGER NOT NULL UNIQUE CHECK(generation>0),
    policy INTEGER NOT NULL CHECK(policy BETWEEN 0 AND 4294967295),
    grant_epoch INTEGER NOT NULL CHECK(grant_epoch BETWEEN 0 AND 4294967295)
) STRICT;
CREATE TABLE s_key_history (
    fingerprint BLOB PRIMARY KEY CHECK(length(fingerprint)=32),
    activation INTEGER NOT NULL REFERENCES s_activation_history(seq)
) STRICT;
CREATE TABLE s_meta (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    activation INTEGER NOT NULL UNIQUE REFERENCES s_activation_history(seq)
) STRICT;
CREATE TABLE s_policy_inputs (
    decision BLOB PRIMARY KEY CHECK(length(decision)=16),
    binding BLOB NOT NULL CHECK(length(binding)=32),
    activation INTEGER NOT NULL REFERENCES s_activation_history(seq)
) STRICT;
CREATE TABLE s_rules (
    id BLOB PRIMARY KEY CHECK(length(id)=16),
    version INTEGER NOT NULL CHECK(version>0),
    used INTEGER NOT NULL CHECK(used BETWEEN 0 AND 3),
    revoked INTEGER NOT NULL CHECK(revoked IN (0,1)),
    FOREIGN KEY(id,version) REFERENCES s_rule_versions(id,version) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE s_rule_versions (
    id BLOB NOT NULL REFERENCES s_rules(id) DEFERRABLE INITIALLY DEFERRED,
    version INTEGER NOT NULL CHECK(version>0),
    approval_event BLOB NOT NULL UNIQUE CHECK(length(approval_event)=16),
    scope BLOB NOT NULL CHECK(length(scope)=32),
    approved_utc INTEGER NOT NULL CHECK(approved_utc>=0),
    expires_utc INTEGER NOT NULL CHECK(expires_utc=approved_utc+600000),
    max_operations INTEGER NOT NULL CHECK(max_operations=3),
    activation INTEGER NOT NULL REFERENCES s_activation_history(seq),
    PRIMARY KEY(id,version)
) STRICT;
CREATE TABLE s_prepared (
    id BLOB PRIMARY KEY CHECK(length(id)=16),
    generation INTEGER NOT NULL CHECK(generation>0),
    connection BLOB NOT NULL CHECK(length(connection)=16),
    broker BLOB NOT NULL CHECK(length(broker)=16),
    worker BLOB NOT NULL CHECK(length(worker)=16 AND worker!=broker),
    wall INTEGER NOT NULL CHECK(wall>=0),
    mono INTEGER NOT NULL CHECK(mono>=0),
    activation INTEGER NOT NULL REFERENCES s_activation_history(seq),
    UNIQUE(id,generation,connection)
) STRICT;
CREATE TABLE s_grants (
    grant_id BLOB PRIMARY KEY REFERENCES grants(id),
    decision BLOB NOT NULL UNIQUE REFERENCES decisions(id),
    prepared BLOB NOT NULL UNIQUE REFERENCES s_prepared(id),
    invocation BLOB NOT NULL UNIQUE CHECK(length(invocation)=16),
    binding BLOB NOT NULL CHECK(length(binding)=32),
    semantic TEXT NOT NULL CHECK(length(semantic)=64),
    scope BLOB NOT NULL CHECK(length(scope)=32),
    activation INTEGER NOT NULL REFERENCES s_activation_history(seq)
) STRICT;
CREATE TABLE s_approval_heads (
    id BLOB PRIMARY KEY CHECK(length(id)=16),
    version INTEGER NOT NULL CHECK(version>0),
    revoked INTEGER NOT NULL CHECK(revoked IN (0,1)),
    FOREIGN KEY(id,version) REFERENCES s_approval_versions(id,version) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE s_approval_versions (
    id BLOB NOT NULL REFERENCES s_approval_heads(id) DEFERRABLE INITIALLY DEFERRED,
    version INTEGER NOT NULL CHECK(version>0),
    grant_id BLOB NOT NULL REFERENCES s_grants(grant_id),
    event BLOB NOT NULL UNIQUE CHECK(length(event)=16),
    actor BLOB NOT NULL CHECK(length(actor)=16),
    challenge BLOB NOT NULL UNIQUE CHECK(length(challenge)=16),
    binding BLOB NOT NULL CHECK(length(binding)=32),
    rule_id BLOB,
    rule_version INTEGER,
    wall INTEGER NOT NULL CHECK(wall>=0),
    mono INTEGER NOT NULL CHECK(mono>=0),
    CHECK((rule_id IS NULL)=(rule_version IS NULL)),
    FOREIGN KEY(rule_id,rule_version) REFERENCES s_rule_versions(id,version),
    PRIMARY KEY(id,version)
) STRICT;
CREATE TABLE s_approval_claims (
    approval BLOB PRIMARY KEY REFERENCES s_approval_heads(id),
    version INTEGER NOT NULL,
    reservation BLOB NOT NULL UNIQUE REFERENCES s_reservations(id) DEFERRABLE INITIALLY DEFERRED,
    grant_id BLOB NOT NULL UNIQUE REFERENCES s_grants(grant_id),
    audit_seq INTEGER NOT NULL UNIQUE REFERENCES s_audit(seq) DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(approval,version) REFERENCES s_approval_versions(id,version)
) STRICT;
CREATE TABLE s_reservations (
    id BLOB PRIMARY KEY CHECK(length(id)=16),
    grant_id BLOB NOT NULL UNIQUE REFERENCES g0_reservations(grant_id),
    approval BLOB NOT NULL UNIQUE REFERENCES s_approval_claims(approval) DEFERRABLE INITIALLY DEFERRED,
    nonce_hash BLOB NOT NULL UNIQUE REFERENCES nonces(hash),
    operation_mac TEXT NOT NULL CHECK(length(operation_mac)=64),
    permit_mac TEXT NOT NULL CHECK(length(permit_mac)=64),
    response_digest BLOB NOT NULL CHECK(length(response_digest)=32),
    issued_wall INTEGER NOT NULL CHECK(issued_wall>=0),
    issued_mono INTEGER NOT NULL CHECK(issued_mono>=0),
    wall INTEGER NOT NULL,
    mono INTEGER NOT NULL,
    ttl INTEGER NOT NULL CHECK(ttl BETWEEN 1 AND 60000),
    activation INTEGER NOT NULL REFERENCES s_activation_history(seq),
    audit_seq INTEGER NOT NULL UNIQUE REFERENCES s_audit(seq) DEFERRABLE INITIALLY DEFERRED,
    CHECK(wall=issued_wall+ttl AND mono=issued_mono+ttl),
    UNIQUE(id,grant_id)
) STRICT;
CREATE TABLE s_launch_once (
    id BLOB PRIMARY KEY CHECK(length(id)=16),
    reservation BLOB NOT NULL UNIQUE,
    grant_id BLOB NOT NULL UNIQUE REFERENCES execution(grant_id),
    response_digest BLOB NOT NULL CHECK(length(response_digest)=32),
    audit_seq INTEGER NOT NULL UNIQUE REFERENCES s_audit(seq) DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(reservation,grant_id) REFERENCES s_reservations(id,grant_id)
) STRICT;
CREATE TABLE s_execution_facts (
    seq INTEGER PRIMARY KEY,
    grant_id BLOB NOT NULL REFERENCES execution(grant_id),
    kind TEXT NOT NULL CHECK(kind IN ('launch_registered','cancel_requested','recovery_unknown')),
    audit_seq INTEGER NOT NULL REFERENCES s_audit(seq) DEFERRABLE INITIALLY DEFERRED,
    UNIQUE(grant_id,kind,audit_seq)
) STRICT;
CREATE TABLE s_invalidated_prepared (
    id BLOB PRIMARY KEY REFERENCES s_prepared(id),
    audit_seq INTEGER NOT NULL REFERENCES s_audit(seq) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE s_audit (
    seq INTEGER PRIMARY KEY CHECK(seq>0),
    activation INTEGER NOT NULL REFERENCES s_activation_history(seq),
    kind TEXT NOT NULL CHECK(kind IN ('activation','policy','register','rule','reserve','consume','revoke_approval','revoke_rule','invalidate','cancel')),
    grant_id BLOB REFERENCES s_grants(grant_id) DEFERRABLE INITIALLY DEFERRED,
    g0_seq INTEGER UNIQUE REFERENCES audit(seq),
    request BLOB UNIQUE REFERENCES s_requests(id) DEFERRABLE INITIALLY DEFERRED,
    event_digest BLOB NOT NULL CHECK(length(event_digest)=32),
    previous_mac TEXT CHECK(length(previous_mac)=64),
    mac TEXT NOT NULL CHECK(length(mac)=64)
) STRICT;
CREATE TABLE s_requests (
    id BLOB PRIMARY KEY CHECK(length(id)=16),
    fingerprint BLOB NOT NULL CHECK(length(fingerprint)=32),
    audit_seq INTEGER NOT NULL UNIQUE REFERENCES s_audit(seq),
    grant_id BLOB REFERENCES s_grants(grant_id),
    kind TEXT NOT NULL,
    activation INTEGER NOT NULL REFERENCES s_activation_history(seq),
    reservation BLOB REFERENCES s_reservations(id),
    launch BLOB REFERENCES s_launch_once(id)
) STRICT;

CREATE TRIGGER s_rule_no_refund BEFORE UPDATE ON s_rules
WHEN NEW.id IS NOT OLD.id OR NEW.used<OLD.used OR NEW.revoked<OLD.revoked
  OR NEW.version<OLD.version OR NEW.used>3
BEGIN SELECT RAISE(ABORT,'S rule history'); END;
CREATE TRIGGER s_approval_no_restore BEFORE UPDATE ON s_approval_heads
WHEN NEW.id IS NOT OLD.id OR NEW.version<OLD.version OR NEW.revoked<OLD.revoked
BEGIN SELECT RAISE(ABORT,'S approval history'); END;
CREATE TRIGGER s_request_namespace BEFORE INSERT ON s_requests
WHEN EXISTS(SELECT 1 FROM audit WHERE request=NEW.id)
BEGIN SELECT RAISE(ABORT,'S global request conflict'); END;
CREATE TRIGGER s_g0_request_namespace BEFORE INSERT ON audit
WHEN NEW.request IS NOT NULL AND (EXISTS(SELECT 1 FROM s_requests WHERE id=NEW.request)
    OR EXISTS(SELECT 1 FROM s_grants WHERE grant_id=NEW.grant_id))
BEGIN SELECT RAISE(ABORT,'S legacy request conflict'); END;

CREATE VIEW s_binding_faults AS
SELECT 'grant' AS kind FROM s_grants s JOIN grants g ON g.id=s.grant_id
 JOIN decisions d ON d.id=g.decision JOIN s_prepared p ON p.id=s.prepared
 WHERE s.decision!=d.id OR s.binding!=d.input_digest OR p.activation!=s.activation
    OR NOT EXISTS(SELECT 1 FROM s_policy_inputs i WHERE i.decision=s.decision
        AND i.binding=s.binding AND i.activation=s.activation)
UNION ALL SELECT 'approval_binding' FROM s_approval_versions a JOIN s_grants s ON s.grant_id=a.grant_id
 WHERE a.binding!=s.binding
UNION ALL SELECT 'approval_head' FROM s_approval_heads h WHERE h.version!=(
 SELECT max(version) FROM s_approval_versions a WHERE a.id=h.id)
UNION ALL SELECT 'rule_head' FROM s_rules h WHERE h.version!=(
 SELECT max(version) FROM s_rule_versions v WHERE v.id=h.id)
UNION ALL SELECT 'claim' FROM s_approval_claims c
 JOIN s_approval_versions a ON a.id=c.approval AND a.version=c.version
 JOIN s_reservations r ON r.id=c.reservation JOIN s_audit u ON u.seq=c.audit_seq
 WHERE a.grant_id!=c.grant_id OR r.grant_id!=c.grant_id OR r.approval!=c.approval
    OR r.audit_seq!=c.audit_seq OR u.kind!='reserve' OR u.grant_id!=c.grant_id
UNION ALL SELECT 'rule' FROM s_rules r WHERE r.used != (
 SELECT count(*) FROM s_approval_claims c JOIN s_approval_versions a
 ON a.id=c.approval AND a.version=c.version WHERE a.rule_id=r.id)
UNION ALL SELECT 'reservation' FROM s_reservations r
 JOIN g0_reservations g ON g.grant_id=r.grant_id JOIN nonces n ON n.hash=r.nonce_hash
 JOIN s_approval_claims c ON c.approval=r.approval
 JOIN s_grants s ON s.grant_id=r.grant_id
 WHERE g.nonce_hash!=r.nonce_hash OR n.grant_id!=r.grant_id OR c.reservation!=r.id
    OR g.wall_deadline!=r.wall OR g.mono_deadline!=r.mono OR s.activation!=r.activation
UNION ALL SELECT 'reserved_group' FROM s_grants s JOIN grants g ON g.id=s.grant_id
 WHERE (g.wall_deadline IS NOT NULL) != EXISTS(SELECT 1 FROM s_reservations r WHERE r.grant_id=g.id)
UNION ALL SELECT 'consume_group' FROM s_grants s JOIN grants g ON g.id=s.grant_id
 WHERE (g.state='consumed') != EXISTS(SELECT 1 FROM s_launch_once l WHERE l.grant_id=g.id)
UNION ALL SELECT 'launch' FROM s_launch_once l JOIN s_audit a ON a.seq=l.audit_seq
 WHERE a.kind!='consume' OR a.grant_id!=l.grant_id OR NOT EXISTS(
 SELECT 1 FROM s_execution_facts f WHERE f.grant_id=l.grant_id AND f.kind='launch_registered' AND f.audit_seq=a.seq)
UNION ALL SELECT 'request' FROM s_requests r JOIN s_audit a ON a.seq=r.audit_seq
 WHERE a.request IS NOT r.id OR a.grant_id IS NOT r.grant_id OR a.kind!=r.kind OR a.activation!=r.activation
    OR (r.kind IN ('reserve','consume'))!=(r.reservation IS NOT NULL)
    OR (r.kind='consume')!=(r.launch IS NOT NULL)
    OR (r.kind='reserve' AND NOT EXISTS(SELECT 1 FROM s_reservations x
         WHERE x.id=r.reservation AND x.grant_id=r.grant_id AND x.audit_seq=r.audit_seq))
    OR (r.kind='consume' AND NOT EXISTS(SELECT 1 FROM s_launch_once x
         WHERE x.id=r.launch AND x.reservation=r.reservation AND x.grant_id=r.grant_id AND x.audit_seq=r.audit_seq))
UNION ALL SELECT 'audit_request' FROM s_audit a WHERE a.request IS NOT NULL AND NOT EXISTS(
 SELECT 1 FROM s_requests r WHERE r.id=a.request AND r.audit_seq=a.seq)
UNION ALL SELECT 'missing_receipt' FROM s_audit a WHERE a.kind IN ('reserve','consume') AND a.request IS NULL
UNION ALL SELECT 'g0_link' FROM s_audit a JOIN audit g ON g.seq=a.g0_seq
 WHERE g.grant_id IS NOT a.grant_id OR g.request IS NOT NULL
    OR (a.kind='reserve' AND (g.kind!='transition' OR g.state!='reserved'))
    OR (a.kind='consume' AND (g.kind!='transition' OR g.state!='consumed'))
UNION ALL SELECT 'g0_audit' FROM audit a JOIN s_grants s ON s.grant_id=a.grant_id
 WHERE NOT (a.kind='transition' AND a.state='created') AND NOT EXISTS(
 SELECT 1 FROM s_audit u WHERE u.g0_seq=a.seq AND u.grant_id=a.grant_id);
CREATE TRIGGER s_activation_history_immutable_update BEFORE UPDATE ON s_activation_history
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_activation_history_immutable_delete BEFORE DELETE ON s_activation_history
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_activation_history_no_replace BEFORE INSERT ON s_activation_history
WHEN EXISTS(SELECT 1 FROM s_activation_history WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_key_history_immutable_update BEFORE UPDATE ON s_key_history
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_key_history_immutable_delete BEFORE DELETE ON s_key_history
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_key_history_no_replace BEFORE INSERT ON s_key_history
WHEN EXISTS(SELECT 1 FROM s_key_history WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_rule_versions_immutable_update BEFORE UPDATE ON s_rule_versions
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_rule_versions_immutable_delete BEFORE DELETE ON s_rule_versions
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_rule_versions_no_replace BEFORE INSERT ON s_rule_versions
WHEN EXISTS(SELECT 1 FROM s_rule_versions WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_prepared_immutable_update BEFORE UPDATE ON s_prepared
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_prepared_immutable_delete BEFORE DELETE ON s_prepared
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_prepared_no_replace BEFORE INSERT ON s_prepared
WHEN EXISTS(SELECT 1 FROM s_prepared WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_grants_immutable_update BEFORE UPDATE ON s_grants
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_grants_immutable_delete BEFORE DELETE ON s_grants
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_grants_no_replace BEFORE INSERT ON s_grants
WHEN EXISTS(SELECT 1 FROM s_grants WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_approval_versions_immutable_update BEFORE UPDATE ON s_approval_versions
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_approval_versions_immutable_delete BEFORE DELETE ON s_approval_versions
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_approval_versions_no_replace BEFORE INSERT ON s_approval_versions
WHEN EXISTS(SELECT 1 FROM s_approval_versions WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_approval_claims_immutable_update BEFORE UPDATE ON s_approval_claims
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_approval_claims_immutable_delete BEFORE DELETE ON s_approval_claims
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_approval_claims_no_replace BEFORE INSERT ON s_approval_claims
WHEN EXISTS(SELECT 1 FROM s_approval_claims WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_reservations_immutable_update BEFORE UPDATE ON s_reservations
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_reservations_immutable_delete BEFORE DELETE ON s_reservations
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_reservations_no_replace BEFORE INSERT ON s_reservations
WHEN EXISTS(SELECT 1 FROM s_reservations WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_launch_once_immutable_update BEFORE UPDATE ON s_launch_once
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_launch_once_immutable_delete BEFORE DELETE ON s_launch_once
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_launch_once_no_replace BEFORE INSERT ON s_launch_once
WHEN EXISTS(SELECT 1 FROM s_launch_once WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_execution_facts_immutable_update BEFORE UPDATE ON s_execution_facts
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_execution_facts_immutable_delete BEFORE DELETE ON s_execution_facts
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_execution_facts_no_replace BEFORE INSERT ON s_execution_facts
WHEN EXISTS(SELECT 1 FROM s_execution_facts WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_invalidated_prepared_immutable_update BEFORE UPDATE ON s_invalidated_prepared
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_invalidated_prepared_immutable_delete BEFORE DELETE ON s_invalidated_prepared
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_invalidated_prepared_no_replace BEFORE INSERT ON s_invalidated_prepared
WHEN EXISTS(SELECT 1 FROM s_invalidated_prepared WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_audit_immutable_update BEFORE UPDATE ON s_audit
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_audit_immutable_delete BEFORE DELETE ON s_audit
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_audit_no_replace BEFORE INSERT ON s_audit
WHEN EXISTS(SELECT 1 FROM s_audit WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_requests_immutable_update BEFORE UPDATE ON s_requests
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_requests_immutable_delete BEFORE DELETE ON s_requests
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_requests_no_replace BEFORE INSERT ON s_requests
WHEN EXISTS(SELECT 1 FROM s_requests WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_policy_inputs_immutable_update BEFORE UPDATE ON s_policy_inputs
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_policy_inputs_immutable_delete BEFORE DELETE ON s_policy_inputs
BEGIN SELECT RAISE(ABORT,'S immutable history'); END;
CREATE TRIGGER s_policy_inputs_no_replace BEFORE INSERT ON s_policy_inputs
WHEN EXISTS(SELECT 1 FROM s_policy_inputs WHERE rowid=NEW.rowid)
BEGIN SELECT RAISE(ABORT,'S history conflict'); END;
CREATE TRIGGER s_meta_no_delete BEFORE DELETE ON s_meta
BEGIN SELECT RAISE(ABORT,'S head history'); END;
CREATE TRIGGER s_rules_no_delete BEFORE DELETE ON s_rules
BEGIN SELECT RAISE(ABORT,'S head history'); END;
CREATE TRIGGER s_approval_heads_no_delete BEFORE DELETE ON s_approval_heads
BEGIN SELECT RAISE(ABORT,'S head history'); END;
