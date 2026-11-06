//! Disposable SQLite process probe. The executable accepts no database path or SQL.
#![forbid(unsafe_code)]

use rusqlite::{Connection, Error, ErrorCode, OpenFlags};
use std::{
    env,
    error::Error as StdError,
    fs::{self, File, OpenOptions, TryLockError},
    os::windows::fs::MetadataExt,
    path::PathBuf,
    thread,
    time::Duration,
};

#[cfg(not(all(target_os = "windows", target_arch = "x86_64", target_env = "msvc")))]
compile_error!("C2 requires Windows x64/MSVC");

const DB: &str = "probe.sqlite3";
const WAL: &str = "probe.sqlite3-wal";
const WRITER_LOCK: &str = "writer.lock";
const SOURCE_ID: &str =
    "2026-10-16 14:45:20 555e07ef33f69a0957d24d8e555b702464a131d6f1607f4c3c1e4551e8e6ca76";
type Result<T> = std::result::Result<T, Box<dyn StdError>>;

fn verify_directory() -> Result<()> {
    let cwd = env::current_dir()?.canonicalize()?;
    if cwd.parent() != Some(&env::temp_dir().canonicalize()?)
        || !cwd
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("agentguard-c2-"))
    {
        return Err("probe requires its disposable C2 directory".into());
    }
    for name in [
        DB,
        WAL,
        "probe.sqlite3-shm",
        "probe.sqlite3-journal",
        WRITER_LOCK,
        "fixture",
        "ready",
        "ready.tmp",
    ] {
        match fs::symlink_metadata(name) {
            Ok(metadata) if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 => {
                return Err("probe file is not a regular local file".into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    if fs::read("fixture")? != b"AgentGuard C2 synthetic fixture\n" {
        return Err("fixture marker is invalid".into());
    }
    Ok(())
}

fn writer_lock() -> Result<Option<File>> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(WRITER_LOCK)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error.into()),
    }
}

fn writer() -> Result<File> {
    writer_lock()?.ok_or_else(|| "another C2 writer holds the lock".into())
}

fn open(flags: OpenFlags) -> Result<Connection> {
    let connection = Connection::open_with_flags(DB, flags)?;
    connection.busy_timeout(Duration::from_millis(150))?;
    connection.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA foreign_keys=ON;")?;
    Ok(connection)
}

fn verify_identity(db: &Connection) -> Result<Vec<String>> {
    let (version, source_id): (String, String) =
        db.query_row("SELECT sqlite_version(), sqlite_source_id()", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
    if version != "3.54.1" || source_id != SOURCE_ID {
        return Err("unexpected SQLite build identity".into());
    }
    let options = db
        .prepare("PRAGMA compile_options")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if !options.iter().any(|option| option == "THREADSAFE=1")
        || options.iter().any(|option| option == "OMIT_WAL")
    {
        return Err("unexpected SQLite compile options".into());
    }
    Ok(options)
}

fn configure_writer(db: &Connection) -> Result<Vec<String>> {
    let options = verify_identity(db)?;
    let mode: String = db.query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    db.execute_batch("PRAGMA synchronous=FULL; PRAGMA wal_autocheckpoint=0;")?;
    let synchronous: u32 = db.query_row("PRAGMA synchronous", [], |row| row.get(0))?;
    let foreign_keys: u32 = db.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    let autocheckpoint: u32 = db.query_row("PRAGMA wal_autocheckpoint", [], |row| row.get(0))?;
    if mode != "wal" || synchronous != 2 || foreign_keys != 1 || autocheckpoint != 0 {
        return Err("writer pragmas did not take effect".into());
    }
    Ok(options)
}

fn hold() -> Result<()> {
    let _lock = writer()?;
    if PathBuf::from(DB).exists() {
        return Err("hold requires an empty fixture".into());
    }
    let db = open(OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?;
    let options = configure_writer(&db)?;
    db.execute_batch(
        "CREATE TABLE probe(id INTEGER PRIMARY KEY, label TEXT NOT NULL, data BLOB NOT NULL);
         BEGIN IMMEDIATE;
         INSERT INTO probe VALUES (1, 'committed', X'01'), (2, 'committed', X'02');
         COMMIT;",
    )?;
    let committed_wal = fs::metadata(WAL)?.len();
    db.execute_batch(
        "PRAGMA cache_size=4; PRAGMA cache_spill=ON;
         BEGIN IMMEDIATE;
         WITH RECURSIVE seq(id) AS (VALUES(100) UNION ALL SELECT id + 1 FROM seq WHERE id < 163)
         INSERT INTO probe SELECT id, 'uncommitted', zeroblob(8192) FROM seq;",
    )?;
    let pending: i64 = db.query_row("SELECT count(*) FROM probe WHERE id >= 100", [], |row| {
        row.get(0)
    })?;
    let uncommitted_wal = fs::metadata(WAL)?.len();
    if uncommitted_wal <= committed_wal || pending != 64 {
        return Err("fault fixture did not spill uncommitted WAL frames".into());
    }
    fs::write(
        "ready.tmp",
        format!(
            "version=3.54.1\nsource_id={SOURCE_ID}\ncompile_options={}\njournal_mode=wal\nsynchronous=2\nforeign_keys=1\nwal_autocheckpoint=0\ncommitted_wal={committed_wal}\nuncommitted_wal={uncommitted_wal}\npending={pending}\n",
            options.join(","),
        ),
    )?;
    fs::rename("ready.tmp", "ready")?;
    thread::sleep(Duration::from_secs(90));
    Err("holder was not terminated by the harness".into())
}

fn observe(db: &Connection) -> Result<()> {
    let integrity: String = db.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err("probe DB integrity check failed".into());
    }
    let mut statement = db.prepare("SELECT id, label, hex(data) FROM probe ORDER BY id LIMIT 4")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let label: String = row.get(1)?;
        let data: String = row.get(2)?;
        println!("{id}|{label}|{data}");
    }
    let total: i64 = db.query_row("SELECT count(*) FROM probe", [], |row| row.get(0))?;
    println!("rows={total}\nintegrity={integrity}");
    Ok(())
}

fn expect_sqlite(error: rusqlite::Result<()>, expected: ErrorCode) -> Result<i32> {
    match error {
        Err(Error::SqliteFailure(code, _)) if code.code == expected => Ok(code.extended_code),
        _ => Err("fault did not produce the expected SQLite error".into()),
    }
}

fn inject_full() -> Result<()> {
    let _lock = writer()?;
    let db = open(OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    configure_writer(&db)?;
    let page_count: u32 = db.query_row("PRAGMA page_count", [], |row| row.get(0))?;
    db.pragma_update(None, "max_page_count", page_count)?;
    let limit: u32 = db.query_row("PRAGMA max_page_count", [], |row| row.get(0))?;
    if limit != page_count {
        return Err("test page limit did not take effect".into());
    }
    let code = expect_sqlite(
        db.execute_batch(
            "BEGIN IMMEDIATE;
             INSERT INTO probe VALUES (3, 'before-full', X'03');
             INSERT INTO probe VALUES (4, 'must-fail', zeroblob(1048576));",
        ),
        ErrorCode::DiskFull,
    )?;
    if !db.is_autocommit() {
        db.execute_batch("ROLLBACK")?;
    }
    println!("sqlite_full={code}\nmax_page_count={limit}");
    Ok(())
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or("missing C2 probe command")?;
    if args.next().is_some() {
        return Err("probe accepts no user paths or SQL".into());
    }
    verify_directory()?;
    match command.as_str() {
        "hold" => hold(),
        "contend" => {
            if writer_lock()?.is_some() {
                return Err("a second writer acquired the fixture lock".into());
            }
            println!("writer_lock=busy");
            Ok(())
        }
        "sqlite-busy" => {
            let db = open(OpenFlags::SQLITE_OPEN_READ_WRITE)?;
            let code = expect_sqlite(db.execute_batch("BEGIN IMMEDIATE"), ErrorCode::DatabaseBusy)?;
            println!("sqlite_busy={code}");
            Ok(())
        }
        "reopen" => {
            let _lock = writer()?;
            let db = open(OpenFlags::SQLITE_OPEN_READ_WRITE)?;
            configure_writer(&db)?;
            observe(&db)
        }
        "read" => {
            let db = open(OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            db.execute_batch("PRAGMA query_only=ON;")?;
            observe(&db)
        }
        "full" => inject_full(),
        _ => Err("unknown C2 probe command".into()),
    }
}

fn main() -> std::process::ExitCode {
    if let Err(error) = run() {
        eprintln!("C2 probe failed: {error}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}
