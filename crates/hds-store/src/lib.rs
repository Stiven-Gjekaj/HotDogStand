//! The workspace file of HotDogStand.
//!
//! The store writes each change and its event in one transaction, so the
//! history always agrees with the data.

mod backup;
pub mod config;
mod migrations;
mod people;
mod snapshot;
mod tickets;

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use rusqlite::{Connection, OptionalExtension, params};
use rusqlite_migration::MigrationDefinitionError;

/// The variable that gives a different path for the workspace file.
pub const WORKSPACE_VAR: &str = "HOTDOGSTAND_WORKSPACE";

/// The mark in the header of each workspace file: "HDST" in ASCII. SQLite
/// keeps it in the `application_id` of the file, so HotDogStand can tell its
/// own files from the files of other programs.
pub const APPLICATION_ID: i32 = 0x4844_5354;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Rule(#[from] hds_core::Error),
    #[error("There is no {what} with the number {id}.")]
    NotFound { what: &'static str, id: i64 },
    #[error("There is already a {what} with the name \"{name}\". Use a different name.")]
    NameTaken { what: &'static str, name: String },
    #[error(
        "A newer version of HotDogStand changed this workspace file. Install the newer version to open it."
    )]
    NewerFile,
    #[error(
        "The data directory of this system is not known. Set {WORKSPACE_VAR} to the path of the workspace file."
    )]
    NoDataDirectory,
    #[error("The file {0} is not a HotDogStand workspace. HotDogStand did not change it.")]
    NotAWorkspace(PathBuf),
    #[error("There is no file at {0}.")]
    NoFile(PathBuf),
    #[error("A backup cannot go in the place of the open workspace. Choose a different file.")]
    BackupOverWorkspace,
    #[error("This workspace holds data. An export goes only into an empty workspace.")]
    NotEmpty,
    #[error("The export does not agree with itself, so nothing was imported: {0}")]
    BadExport(String),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Migration(rusqlite_migration::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<rusqlite_migration::Error> for Error {
    fn from(error: rusqlite_migration::Error) -> Self {
        match error {
            rusqlite_migration::Error::MigrationDefinition(
                MigrationDefinitionError::DatabaseTooFarAhead,
            ) => Error::NewerFile,
            error => Error::Migration(error),
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Gives the path of the workspace file: the value of
/// [`WORKSPACE_VAR`] when it is set, or the file in the data directory of the
/// system.
pub fn workspace_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(WORKSPACE_VAR).filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    Ok(data_dir()?.join("workspace.db"))
}

/// True when [`WORKSPACE_VAR`] names the workspace. Then the application
/// opens that file and not the one that it opened last.
pub fn workspace_is_given() -> bool {
    std::env::var_os(WORKSPACE_VAR).is_some_and(|p| !p.is_empty())
}

/// The directory of the data of HotDogStand on this system.
pub(crate) fn data_dir() -> Result<PathBuf> {
    let base = directories::BaseDirs::new().ok_or(Error::NoDataDirectory)?;
    // Linux names its data directories in lower case. Windows and macOS use
    // the name of the application.
    let name = if cfg!(target_os = "linux") {
        "hotdogstand"
    } else {
        "HotDogStand"
    };
    Ok(base.data_dir().join(name))
}

type Clock = Box<dyn Fn() -> Timestamp>;

/// An open workspace file.
pub struct Store {
    conn: Connection,
    clock: Clock,
}

impl Store {
    /// Opens the workspace file at `path`. It makes the file and its
    /// directory when they are not there, and brings an older file up to date.
    /// It refuses a file of another program, and does not change it.
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Store::prepare(Connection::open(path)?, path)
    }

    /// Opens a workspace file that is there already. A path with no file is
    /// an error, so that a wrong path does not make an empty workspace.
    pub fn open_existing(path: &Path) -> Result<Store> {
        if !path.is_file() {
            return Err(Error::NoFile(path.to_owned()));
        }
        Store::prepare(Connection::open(path)?, path)
    }

    /// Opens a workspace that lives in memory and goes when the store goes.
    pub fn open_in_memory() -> Result<Store> {
        Store::prepare(Connection::open_in_memory()?, Path::new(":memory:"))
    }

    fn prepare(mut conn: Connection, path: &Path) -> Result<Store> {
        check_workspace(&conn, path)?;
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        migrations::migrations().to_latest(&mut conn)?;
        conn.pragma_update(None, "application_id", APPLICATION_ID)?;
        Ok(Store {
            conn,
            clock: Box::new(Timestamp::now),
        })
    }

    /// Replaces the clock. A test uses a clock that it controls.
    pub fn with_clock(mut self, clock: impl Fn() -> Timestamp + 'static) -> Store {
        self.clock = Box::new(clock);
        self
    }

    fn now(&self) -> Timestamp {
        (self.clock)()
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM setting WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    pub(crate) fn settings(&self) -> Result<std::collections::BTreeMap<String, String>> {
        let mut statement = self.conn.prepare("SELECT key, value FROM setting")?;
        let pairs = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(pairs.collect::<rusqlite::Result<_>>()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO setting (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

/// Refuses a file that is not a workspace, before anything writes to it.
///
/// A file with the mark is a workspace. An empty file becomes one. A file
/// with no mark and the tables of a workspace comes from version 0.1, which
/// wrote no mark, and gets the mark. Any other file belongs to another
/// program.
fn check_workspace(conn: &Connection, path: &Path) -> Result<()> {
    let not_a_workspace = || Error::NotAWorkspace(path.to_owned());
    let id: i32 = conn
        .pragma_query_value(None, "application_id", |row| row.get(0))
        .map_err(|error| match error.sqlite_error_code() {
            Some(rusqlite::ErrorCode::NotADatabase) => not_a_workspace(),
            _ => Error::Sqlite(error),
        })?;
    if id == APPLICATION_ID {
        return Ok(());
    }
    if id != 0 {
        return Err(not_a_workspace());
    }
    let tables: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let ours = ["ticket", "person", "label", "event", "setting"];
    if tables.is_empty() || ours.iter().all(|t| tables.iter().any(|n| n == t)) {
        Ok(())
    } else {
        Err(not_a_workspace())
    }
}

/// Reads a timestamp that the store wrote.
fn parse_time(text: &str) -> rusqlite::Result<Timestamp> {
    text.parse().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

/// Gives a [`Error::NameTaken`] for a UNIQUE constraint that fails, and the
/// error as it is for any other failure.
fn name_taken(what: &'static str, name: &str) -> impl FnOnce(rusqlite::Error) -> Error {
    move |error| match error.sqlite_error_code() {
        Some(rusqlite::ErrorCode::ConstraintViolation) if error.to_string().contains("UNIQUE") => {
            Error::NameTaken {
                what,
                name: name.to_owned(),
            }
        }
        _ => Error::Sqlite(error),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    /// A store in memory with a clock that starts at second 1000 and moves one
    /// second each time the store reads it.
    pub fn store() -> Store {
        let second = Rc::new(Cell::new(1000));
        Store::open_in_memory().unwrap().with_clock(move || {
            let now = second.get();
            second.set(now + 1);
            Timestamp::from_second(now).unwrap()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_file_and_its_directory_are_made() {
        let dir = std::env::temp_dir().join(format!("hds-test-{}", std::process::id()));
        let path = dir.join("nested").join("workspace.db");
        let _ = std::fs::remove_dir_all(&dir);

        Store::open(&path).unwrap().set_setting("k", "v").unwrap();
        let again = Store::open(&path).unwrap();
        assert_eq!(again.setting("k").unwrap().as_deref(), Some("v"));

        drop(again);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_from_a_newer_version_is_refused() {
        let dir = std::env::temp_dir().join(format!("hds-newer-{}", std::process::id()));
        let path = dir.join("workspace.db");
        let _ = std::fs::remove_dir_all(&dir);
        drop(Store::open(&path).unwrap());

        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        drop(conn);

        assert!(matches!(Store::open(&path), Err(Error::NewerFile)));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hds-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn application_id(path: &Path) -> i32 {
        Connection::open(path)
            .unwrap()
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn a_new_file_gets_the_mark() {
        let dir = temp_dir("mark");
        let path = dir.join("new.hotdogstand");
        drop(Store::open(&path).unwrap());
        assert_eq!(application_id(&path), APPLICATION_ID);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_database_of_another_program_is_refused_and_not_changed() {
        let dir = temp_dir("foreign");
        let path = dir.join("photos.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE photo (id INTEGER PRIMARY KEY, name TEXT);")
            .unwrap();
        drop(conn);
        let before = std::fs::read(&path).unwrap();

        for result in [Store::open(&path), Store::open_existing(&path)] {
            assert!(matches!(result, Err(Error::NotAWorkspace(ref p)) if p == &path));
        }
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_database_with_another_mark_is_refused() {
        let dir = temp_dir("other-mark");
        let path = dir.join("other.db");
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "application_id", 7).unwrap();
        drop(conn);
        assert!(matches!(Store::open(&path), Err(Error::NotAWorkspace(_))));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_that_is_not_a_database_is_refused_and_not_changed() {
        let dir = temp_dir("text");
        let path = dir.join("notes.db");
        let text = "These are my notes, not a database. ".repeat(200);
        std::fs::write(&path, &text).unwrap();
        assert!(matches!(Store::open(&path), Err(Error::NotAWorkspace(_))));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_workspace_of_version_0_1_opens_and_gets_the_mark() {
        let dir = temp_dir("old");
        let path = dir.join("workspace.db");
        let store = Store::open(&path).unwrap();
        store.set_setting("workspace_name", "Office").unwrap();
        drop(store);
        // Version 0.1 wrote no mark.
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "application_id", 0).unwrap();
        drop(conn);

        let store = Store::open_existing(&path).unwrap();
        assert_eq!(
            store.setting("workspace_name").unwrap().as_deref(),
            Some("Office")
        );
        drop(store);
        assert_eq!(application_id(&path), APPLICATION_ID);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn open_existing_does_not_make_a_file() {
        let dir = temp_dir("missing");
        let path = dir.join("typo.hotdogstand");
        assert!(matches!(Store::open_existing(&path), Err(Error::NoFile(ref p)) if p == &path));
        assert!(!path.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_setting_is_replaced() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.setting("workspace_name").unwrap(), None);
        store.set_setting("workspace_name", "One").unwrap();
        store.set_setting("workspace_name", "Two").unwrap();
        assert_eq!(
            store.setting("workspace_name").unwrap().as_deref(),
            Some("Two")
        );
    }

    #[test]
    fn foreign_keys_are_on() {
        let store = Store::open_in_memory().unwrap();
        let on: bool = store
            .conn
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .unwrap();
        assert!(on);
    }
}
