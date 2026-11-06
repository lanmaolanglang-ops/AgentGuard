CREATE TABLE observations (
    ingest_seq INTEGER PRIMARY KEY,
    source BLOB NOT NULL CHECK(length(source) = 16),
    event_id BLOB NOT NULL CHECK(length(event_id) = 16),
    source_seq INTEGER NOT NULL CHECK(source_seq BETWEEN 1 AND 9007199254740991),
    digest BLOB NOT NULL CHECK(length(digest) = 32),
    UNIQUE(source, event_id),
    UNIQUE(source, source_seq)
) STRICT;
