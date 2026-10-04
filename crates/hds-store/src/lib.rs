//! The workspace file of HotDogStand.
//!
//! The store writes each change and its event in one transaction, so the
//! history always agrees with the data.

mod migrations;
mod people;
mod tickets;

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use rusqlite::{Connection, OptionalExtension, params};
use rusqlite_migration::MigrationDefinitionError;

/// The variable that gives a different path for the workspace file.
pub const WORKSPACE_VAR: &str = "HOTDOGSTAND_WORKSPACE";

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
    let base = directories::BaseDirs::new().ok_or(Error::NoDataDirectory)?;
    // Linux names its data directories in lower case. Windows and macOS use
    // the name of the application.
    let name = if cfg!(target_os = "linux") {
        "hotdogstand"
    } else {
        "HotDogStand"
    };
    Ok(base.data_dir().join(name).join("workspace.db"))
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
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Store::prepare(Connection::open(path)?)
    }

    /// Opens a workspace that lives in memory and goes when the store goes.
    pub fn open_in_memory() -> Result<Store> {
        Store::prepare(Connection::open_in_memory()?)
    }

    fn prepare(mut conn: Connection) -> Result<Store> {
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        migrations::migrations().to_latest(&mut conn)?;
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

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO setting (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
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
