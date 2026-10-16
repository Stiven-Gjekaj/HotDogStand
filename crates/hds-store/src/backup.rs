//! A copy of the workspace file while it is open.

use std::path::Path;

use crate::{Error, Result, Store};

impl Store {
    /// Copies the workspace to the file at `to` with the backup API of
    /// SQLite, so the copy agrees with itself while the workspace is open.
    /// The copy goes to a new file first and then takes the place of `to`, so
    /// a failure leaves the old file at `to` as it was.
    pub fn back_up(&self, to: &Path) -> Result<()> {
        if let Some(open) = self.conn.path().filter(|p| !p.is_empty())
            && same_file(Path::new(open), to)
        {
            return Err(Error::BackupOverWorkspace);
        }
        let mut next = to.as_os_str().to_owned();
        next.push(".new");
        let next = std::path::PathBuf::from(next);
        let _ = std::fs::remove_file(&next);
        let result = self
            .conn
            .backup(rusqlite::MAIN_DB, &next, None)
            .map_err(Error::from)
            .and_then(|()| Ok(std::fs::rename(&next, to)?));
        if result.is_err() {
            let _ = std::fs::remove_file(&next);
        }
        result
    }
}

/// True when the two paths name the same file. A file that is not there yet
/// is not the open workspace.
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use hds_core::{NewTicket, Title};

    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("hds-backup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_backup_holds_the_workspace() {
        let dir = temp_dir("copy");
        let mut store = Store::open(&dir.join("work.hotdogstand")).unwrap();
        store.set_setting("workspace_name", "Office").unwrap();
        store
            .create_ticket(NewTicket::new(Title::new("a").unwrap()))
            .unwrap();
        let to = dir.join("copy.hotdogstand");
        store.back_up(&to).unwrap();

        // A change after the backup is not in the copy.
        store
            .create_ticket(NewTicket::new(Title::new("b").unwrap()))
            .unwrap();
        let copy = Store::open_existing(&to).unwrap();
        assert_eq!(copy.tickets().unwrap().len(), 1);
        assert_eq!(
            copy.setting("workspace_name").unwrap().as_deref(),
            Some("Office")
        );
        assert!(!dir.join("copy.hotdogstand.new").exists());
        drop(copy);
        drop(store);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_backup_replaces_an_old_backup() {
        let dir = temp_dir("replace");
        let to = dir.join("copy.hotdogstand");
        std::fs::write(&to, "old").unwrap();
        let store = Store::open(&dir.join("work.hotdogstand")).unwrap();
        store.back_up(&to).unwrap();
        assert!(Store::open_existing(&to).is_ok());
        drop(store);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_backup_over_the_open_workspace_is_refused() {
        let dir = temp_dir("self");
        let path = dir.join("work.hotdogstand");
        let mut store = Store::open(&path).unwrap();
        store
            .create_ticket(NewTicket::new(Title::new("a").unwrap()))
            .unwrap();
        let same = dir.join(".").join("work.hotdogstand");
        assert!(matches!(
            store.back_up(&same),
            Err(Error::BackupOverWorkspace)
        ));
        assert_eq!(store.tickets().unwrap().len(), 1);
        drop(store);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
