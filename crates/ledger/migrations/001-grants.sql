CREATE TABLE meta (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    boot BLOB NOT NULL CHECK(length(boot) = 16),
    policy INTEGER NOT NULL CHECK(policy >= 0),
    grant_epoch INTEGER NOT NULL CHECK(grant_epoch >= 0),
    archived INTEGER NOT NULL CHECK(archived IN (0, 1))
) STRICT;

CREATE TABLE decisions (
    id BLOB PRIMARY KEY CHECK(length(id) = 16),
    input_digest BLOB NOT NULL CHECK(length(input_digest) = 32),
    boot BLOB NOT NULL CHECK(length(boot) = 16),
    policy INTEGER NOT NULL CHECK(policy >= 0),
    grant_epoch INTEGER NOT NULL CHECK(grant_epoch >= 0),
    eligible INTEGER NOT NULL CHECK(eligible IN (0, 1))
) STRICT;

CREATE TABLE grants (
    id BLOB PRIMARY KEY CHECK(length(id) = 16),
    decision BLOB NOT NULL UNIQUE REFERENCES decisions(id),
    state TEXT NOT NULL CHECK(state IN (
        'created','pending','reserved','consumed','revoked','expired','invalidated','failed'
    )),
    wall_deadline INTEGER CHECK(wall_deadline >= 0),
    mono_deadline INTEGER CHECK(mono_deadline >= 0),
    CHECK((wall_deadline IS NULL) = (mono_deadline IS NULL)),
    CHECK(state NOT IN ('reserved','consumed') OR wall_deadline IS NOT NULL)
) STRICT;
CREATE INDEX grants_state ON grants(state);

CREATE TABLE nonces (
    hash BLOB PRIMARY KEY CHECK(length(hash) = 32),
    grant_id BLOB NOT NULL UNIQUE REFERENCES grants(id)
) STRICT;

CREATE TABLE execution (
    grant_id BLOB PRIMARY KEY REFERENCES grants(id),
    state TEXT NOT NULL CHECK(state IN ('unknown','failed')),
    cancel_requested INTEGER NOT NULL CHECK(cancel_requested IN (0, 1))
) STRICT;

CREATE TABLE audit (
    seq INTEGER PRIMARY KEY,
    grant_id BLOB NOT NULL REFERENCES grants(id),
    kind TEXT NOT NULL CHECK(kind IN ('transition','cancel_requested','worker_failed')),
    previous TEXT,
    state TEXT NOT NULL CHECK(state IN (
        'created','pending','reserved','consumed','revoked','expired','invalidated','failed'
    )),
    request BLOB UNIQUE CHECK(length(request) = 16),
    fingerprint BLOB CHECK(length(fingerprint) = 32),
    CHECK((request IS NULL) = (fingerprint IS NULL)),
    CHECK((kind = 'transition' AND coalesce(((previous IS NULL AND state = 'created') OR
           (previous = 'created' AND state = 'pending') OR
           (previous = 'pending' AND state = 'reserved') OR
           (previous = 'reserved' AND state = 'consumed') OR
           (previous IN ('created','pending','reserved') AND
            state IN ('revoked','expired','invalidated','failed'))),0)) OR
          (kind <> 'transition' AND previous IS NULL AND state = 'consumed'))
) STRICT;
CREATE INDEX audit_grant_sequence ON audit(grant_id, seq);
CREATE UNIQUE INDEX audit_once ON audit(grant_id, kind, state);

CREATE TRIGGER grant_transition BEFORE UPDATE OF state ON grants
WHEN NOT (
    (OLD.state = 'created' AND NEW.state = 'pending') OR
    (OLD.state = 'pending' AND NEW.state = 'reserved') OR
    (OLD.state = 'reserved' AND NEW.state = 'consumed') OR
    (OLD.state IN ('created','pending','reserved') AND
     NEW.state IN ('revoked','expired','invalidated','failed'))
)
BEGIN SELECT RAISE(ABORT, 'invalid grant transition'); END;
CREATE TRIGGER immutable_decision_update BEFORE UPDATE ON decisions
BEGIN SELECT RAISE(ABORT, 'immutable decision'); END;
CREATE TRIGGER immutable_decision_delete BEFORE DELETE ON decisions
BEGIN SELECT RAISE(ABORT, 'immutable decision'); END;
CREATE TRIGGER immutable_nonce_update BEFORE UPDATE ON nonces
BEGIN SELECT RAISE(ABORT, 'immutable nonce'); END;
CREATE TRIGGER immutable_nonce_delete BEFORE DELETE ON nonces
BEGIN SELECT RAISE(ABORT, 'immutable nonce'); END;
CREATE TRIGGER immutable_audit_update BEFORE UPDATE ON audit
BEGIN SELECT RAISE(ABORT, 'append-only audit'); END;
CREATE TRIGGER immutable_audit_delete BEFORE DELETE ON audit
BEGIN SELECT RAISE(ABORT, 'append-only audit'); END;
