use crate::{AuditKind, Epoch, Error, Id, Receipt, Result, State};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction,
    backup::{Backup, StepResult},
};
use std::{
    env,
    fs::{self, File, OpenOptions, TryLockError},
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf, Prefix},
    time::Duration,
};

pub(crate) const APPLICATION_ID: i64 = 0x4147_4C44;
pub(crate) const VERSION: i64 = 3;
pub(crate) const V1: &str = include_str!("../migrations/001-grants.sql");
const V2: &str = include_str!("../migrations/002-events.sql");
const V3: &str = include_str!("../migrations/003-g0-integrity.sql");
pub(crate) const V4: &str = include_str!("../migrations/004-s-atomic.sql");
pub(crate) const DB: &str = "ledger.sqlite3";
const ARCHIVE: &str = "history.sqlite3";
const STAGING: &str = "history.pending";
pub(crate) const MARKER: &[u8] = b"AgentGuard I05 synthetic ledger\n";
const SOURCE: &str =
    "2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76";

pub(crate) fn fixture_dir(path: &Path) -> Result<PathBuf> {
    let temp = env::temp_dir().canonicalize().map_err(|_| Error::Scope)?;
    let root = path.canonicalize().map_err(|_| Error::Scope)?;
    let local = matches!(
        root.components().next(),
        Some(std::path::Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
    );
    if !local
        || root.parent() != Some(temp.as_path())
        || !root
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("agentguard-ledger-"))
        || fs::symlink_metadata(path)
            .map_err(|_| Error::Scope)?
            .file_attributes()
            & 0x400
            != 0
    {
        return Err(Error::Scope);
    }
    for name in [
        DB,
        "ledger.sqlite3-wal",
        "ledger.sqlite3-shm",
        "ledger.sqlite3-journal",
        ARCHIVE,
        "history.sqlite3-wal",
        "history.sqlite3-shm",
        "history.sqlite3-journal",
        STAGING,
        "history.pending-wal",
        "history.pending-shm",
        "history.pending-journal",
        "writer.lock",
        "fixture",
    ] {
        match fs::symlink_metadata(root.join(name)) {
            Ok(info) if !info.is_file() || info.file_attributes() & 0x400 != 0 => {
                return Err(Error::Scope);
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(Error::Scope),
        }
    }
    if fs::read(root.join("fixture")).map_err(|_| Error::Scope)? != MARKER {
        return Err(Error::Scope);
    }
    Ok(root)
}

pub(crate) fn lock(root: &Path) -> Result<File> {
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("writer.lock"))
        .map_err(|_| Error::Storage)?;
    match lock.try_lock() {
        Ok(()) => Ok(lock),
        Err(TryLockError::WouldBlock) => Err(Error::Busy),
        Err(TryLockError::Error(_)) => Err(Error::Storage),
    }
}

pub(crate) fn connection(path: &Path, flags: OpenFlags) -> Result<Connection> {
    let db = Connection::open_with_flags(path, flags)?;
    db.busy_timeout(Duration::from_millis(150))?;
    let (version, source, omitted, enabled): (String, String, bool, bool) = db.query_row(
        "SELECT sqlite_version(), sqlite_source_id(),
         sqlite_compileoption_used('OMIT_LOAD_EXTENSION'),
         sqlite_compileoption_used('ENABLE_LOAD_EXTENSION')",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    if version != "3.54.1" || source != SOURCE || !omitted || enabled {
        return Err(Error::Build);
    }
    db.execute_batch(
        "PRAGMA trusted_schema=OFF; PRAGMA foreign_keys=ON; PRAGMA recursive_triggers=ON;",
    )?;
    for (pragma, expected) in [
        ("trusted_schema", 0),
        ("foreign_keys", 1),
        ("recursive_triggers", 1),
    ] {
        if db.pragma_query_value(None, pragma, |r| r.get::<_, i64>(0))? != expected {
            return Err(Error::Storage);
        }
    }
    Ok(db)
}

pub(crate) fn configure_writer(db: &Connection) -> Result<()> {
    let mode: String = db.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
    db.execute_batch("PRAGMA synchronous=FULL; PRAGMA wal_autocheckpoint=1000;")?;
    if mode != "wal"
        || db.query_row::<i64, _, _>("PRAGMA synchronous", [], |r| r.get(0))? != 2
        || db.query_row::<i64, _, _>("PRAGMA foreign_keys", [], |r| r.get(0))? != 1
    {
        return Err(Error::Storage);
    }
    Ok(())
}

pub(crate) fn version(db: &Connection) -> Result<i64> {
    let version = recognized_version(db)?;
    if version > VERSION {
        return Err(Error::Version);
    }
    Ok(version)
}

pub(crate) fn recognized_version(db: &Connection) -> Result<i64> {
    let id: i64 = db.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if id != APPLICATION_ID || !(1..=4).contains(&version) {
        return Err(Error::Version);
    }
    Ok(version)
}

pub(crate) fn migrate_legacy(tx: &Transaction<'_>, previous: i64, epoch: Epoch) -> Result<()> {
    if previous == 0 {
        tx.pragma_update(None, "application_id", APPLICATION_ID)?;
        tx.execute_batch(V1)?;
        tx.execute(
            "INSERT INTO meta VALUES(1,?,?,?,0)",
            (epoch.boot, epoch.policy, epoch.grant),
        )?;
    }
    if previous < 2 {
        tx.execute_batch(V2)?;
        tx.pragma_update(None, "user_version", 2)?;
    }
    Ok(())
}

pub(crate) fn install_g0(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch(V3)?;
    tx.pragma_update(None, "user_version", VERSION)?;
    Ok(())
}

pub(crate) fn epoch(db: &Connection) -> Result<(Epoch, bool)> {
    Ok(db.query_row(
        "SELECT boot,policy,grant_epoch,archived FROM meta WHERE singleton=1",
        [],
        |r| {
            Ok((
                Epoch {
                    boot: r.get(0)?,
                    policy: r.get(1)?,
                    grant: r.get(2)?,
                },
                r.get(3)?,
            ))
        },
    )?)
}

pub(crate) fn integrity(db: &Connection) -> Result<()> {
    let structural: String = db.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if structural != "ok" {
        return Err(Error::Corrupt);
    }
    let version = recognized_version(db)?;
    // Compare to the schema SQLite creates from the checked-in migrations.
    let reference = Connection::open_in_memory()?;
    reference.execute_batch(V1)?;
    if version >= 2 {
        reference.execute_batch(V2)?;
    }
    if version >= 3 {
        reference.execute_batch(V3)?;
    }
    if version == 4 {
        reference.execute_batch(V4)?;
    }
    if schema_objects(db)? != schema_objects(&reference)? {
        return Err(Error::Corrupt);
    }
    domain_integrity(db)?;
    if version == 4 {
        crate::s::domain_integrity(db)?;
    }
    Ok(())
}

pub(crate) fn domain_integrity(db: &Connection) -> Result<()> {
    if db.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(Error::Corrupt);
    }
    let invalid: bool = db.query_row(
        "WITH steps AS (
           SELECT grant_id, previous, lag(state) OVER(PARTITION BY grant_id ORDER BY seq) AS prior
           FROM audit WHERE kind='transition'
         )
         SELECT EXISTS(SELECT 1 FROM audit WHERE seq<=0)
            OR EXISTS(SELECT 1 FROM steps WHERE previous IS NOT prior)
            OR EXISTS(SELECT 1 FROM grants g JOIN decisions d ON d.id=g.decision WHERE
                g.state IS NOT (SELECT a.state FROM audit a WHERE a.grant_id=g.id AND a.kind='transition'
                                ORDER BY a.seq DESC LIMIT 1)
                OR (g.wall_deadline IS NOT NULL) != EXISTS(SELECT 1 FROM nonces n WHERE n.grant_id=g.id)
                OR (g.wall_deadline IS NOT NULL) != EXISTS(SELECT 1 FROM audit a
                    WHERE a.grant_id=g.id AND a.kind='transition' AND a.state='reserved')
                OR (g.state='consumed') != EXISTS(SELECT 1 FROM execution e WHERE e.grant_id=g.id)
                OR (g.wall_deadline IS NOT NULL AND d.eligible != 1))
            OR EXISTS(SELECT 1 FROM decisions d WHERE NOT EXISTS(SELECT 1 FROM grants g WHERE g.decision=d.id))
            OR EXISTS(SELECT 1 FROM execution e WHERE
                e.cancel_requested != EXISTS(SELECT 1 FROM audit a WHERE a.grant_id=e.grant_id AND a.kind='cancel_requested')
                OR (e.state='failed') != EXISTS(SELECT 1 FROM audit a WHERE a.grant_id=e.grant_id AND a.kind='worker_failed'))
            OR EXISTS(SELECT 1 FROM audit a WHERE a.kind!='transition' AND NOT EXISTS(
                SELECT 1 FROM audit b WHERE b.grant_id=a.grant_id AND b.kind='transition'
                 AND b.state='consumed' AND b.seq<a.seq))",
        [],
        |r| r.get(0),
    )?;
    if invalid {
        return Err(Error::Corrupt);
    }
    if recognized_version(db)? >= 3
        && db.query_row::<bool, _, _>(
            "SELECT NOT EXISTS(SELECT 1 FROM g0_cutover WHERE id=1)
             OR EXISTS(SELECT 1 FROM g0_binding_faults)",
            [],
            |r| r.get(0),
        )?
    {
        return Err(Error::Corrupt);
    }
    Ok(())
}

fn schema_objects(db: &Connection) -> Result<Vec<(String, String, String)>> {
    Ok(db
        .prepare(
            "SELECT type,name,sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY type,name",
        )?
        .query_map([], |r| {
            let sql: String = r.get(2)?;
            Ok((r.get(0)?, r.get(1)?, sql.replace("\r\n", "\n")))
        })?
        .collect::<rusqlite::Result<_>>()?)
}

pub(crate) fn find_receipt(
    db: &Connection,
    request: Id,
) -> Result<Option<(Receipt, crate::Digest)>> {
    Ok(db
        .query_row(
            "SELECT seq,grant_id,state,kind,fingerprint FROM audit WHERE request=?",
            [request],
            |r| {
                Ok((
                    Receipt {
                        sequence: r.get(0)?,
                        grant: r.get(1)?,
                        state: r.get(2)?,
                        kind: r.get(3)?,
                    },
                    r.get(4)?,
                ))
            },
        )
        .optional()?)
}

pub struct Inspection(pub(crate) Connection);

impl Inspection {
    pub fn open(path: &Path) -> Result<Self> {
        Self::read(&fixture_dir(path)?.join(DB), false)
    }

    fn read(path: &Path, archive: bool) -> Result<Self> {
        let db = connection(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        db.execute_batch("PRAGMA query_only=ON;")?;
        recognized_version(&db)?;
        if epoch(&db)?.1 != archive {
            return Err(Error::Archived);
        }
        Ok(Self(db))
    }

    pub fn verify(&self) -> Result<()> {
        let snapshot = self.0.unchecked_transaction()?;
        integrity(&snapshot)?;
        snapshot.commit()?;
        Ok(())
    }

    pub fn epoch(&self) -> Result<Epoch> {
        Ok(epoch(&self.0)?.0)
    }

    pub fn grant(&self, id: Id) -> Result<Option<State>> {
        Ok(self
            .0
            .query_row("SELECT state FROM grants WHERE id=?", [id], |r| r.get(0))
            .optional()?)
    }

    pub fn execution(&self, id: Id) -> Result<Option<(String, bool)>> {
        Ok(self
            .0
            .query_row(
                "SELECT state,cancel_requested FROM execution WHERE grant_id=?",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?)
    }

    pub fn audit(&self, id: Id) -> Result<Vec<(AuditKind, State)>> {
        Ok(self
            .0
            .prepare("SELECT kind,state FROM audit WHERE grant_id=? ORDER BY seq")?
            .query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn reconcile(&self, request: Id) -> Result<Option<Receipt>> {
        Ok(find_receipt(&self.0, request)?.map(|(receipt, _)| receipt))
    }

    pub fn source_gaps(&self, source: Id) -> Result<i64> {
        Ok(self.0.query_row(
            "SELECT coalesce(max(source_seq),0)-count(*) FROM observations WHERE source=?",
            [source],
            |r| r.get(0),
        )?)
    }
}

pub struct Archive(Inspection);

impl Archive {
    pub fn open(path: &Path) -> Result<Self> {
        let view = Inspection::read(&fixture_dir(path)?.join(ARCHIVE), true)?;
        view.verify()?;
        Ok(Self(view))
    }

    pub fn history(&self) -> &Inspection {
        &self.0
    }
}

pub(crate) fn archive(source: &Connection, path: &Path) -> Result<()> {
    let root = fixture_dir(path)?;
    let _lock = lock(&root)?;
    if [DB, ARCHIVE, STAGING]
        .iter()
        .any(|name| root.join(name).exists())
    {
        return Err(Error::Conflict);
    }
    let stage = root.join(STAGING);
    File::create_new(&stage).map_err(|_| Error::Storage)?;
    let mut destination = connection(&stage, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    backup_snapshot(source, &mut destination)?;
    let mode: String = destination.query_row("PRAGMA journal_mode=DELETE", [], |r| r.get(0))?;
    if mode != "delete" {
        return Err(Error::Storage);
    }
    destination
        .execute_batch("PRAGMA synchronous=FULL; UPDATE meta SET archived=1 WHERE singleton=1;")?;
    integrity(&destination)?;
    destination
        .close()
        .map_err(|(_, error)| Error::from(error))?;
    fs::rename(stage, root.join(ARCHIVE)).map_err(|_| Error::Storage)?;
    Ok(())
}

pub(crate) fn backup_snapshot(source: &Connection, destination: &mut Connection) -> Result<()> {
    let backup = Backup::new(source, destination)?;
    for _ in 0..4096 {
        match backup.step(64)? {
            StepResult::Done => return Ok(()),
            StepResult::More => {}
            _ => return Err(Error::Busy),
        }
    }
    Err(Error::Storage)
}
