//! Opens the workspace files and reads the JSON exports that each released
//! version wrote. From 1.0.0, each 1.x release must open them.
//!
//! scripts/make-fixtures.sh makes the files in `tests/fixtures/<version>/`
//! from the code of each release. The test copies each workspace file before
//! it opens it, because an open brings the file up to date.

// A test stops at the first fault, so the helpers unwrap as the tests do.
#![allow(clippy::unwrap_used)]

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use hds_core::export::{Snapshot, read_json};
use hds_store::Store;

/// The mark of a workspace file, "HDST".
const APPLICATION_ID: i32 = 0x4844_5354;

fn fixtures() -> Vec<(String, PathBuf)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut found: Vec<(String, PathBuf)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .map(|path| {
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                path,
            )
        })
        .collect();
    found.sort();
    found
}

fn read_export(dir: &Path) -> Snapshot {
    read_json(BufReader::new(File::open(dir.join("export.json")).unwrap())).unwrap()
}

/// Copies the workspace file of a fixture to a new directory.
fn copy_workspace(version: &str, dir: &Path) -> PathBuf {
    let temp = std::env::temp_dir().join(format!("hds-fixture-{version}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).unwrap();
    let path = temp.join("workspace.hotdogstand");
    std::fs::copy(dir.join("workspace.hotdogstand"), &path).unwrap();
    path
}

/// A snapshot without the time of the export, which is different each time.
fn without_time(mut snapshot: Snapshot, like: &Snapshot) -> Snapshot {
    snapshot.exported_at = like.exported_at;
    snapshot
}

#[test]
fn each_release_has_a_fixture() {
    let versions: Vec<String> = fixtures().into_iter().map(|(v, _)| v).collect();
    for release in ["0.1.0", "0.1.1", "0.2.0", "1.0.0"] {
        assert!(
            versions.iter().any(|v| v == release),
            "no fixture for {release}"
        );
    }
}

#[test]
fn the_workspace_of_each_release_opens_with_all_its_data() {
    for (version, dir) in fixtures() {
        let export = read_export(&dir);
        // The fixture holds data in each table, so the comparison below
        // compares something.
        assert!(!export.tickets.is_empty(), "{version}: no tickets");
        assert!(!export.people.is_empty(), "{version}: no people");
        assert!(!export.labels.is_empty(), "{version}: no labels");
        assert!(!export.comments.is_empty(), "{version}: no comments");
        assert!(!export.events.is_empty(), "{version}: no events");

        let path = copy_workspace(&version, &dir);
        let store = Store::open_existing(&path)
            .unwrap_or_else(|error| panic!("{version}: the workspace does not open: {error}"));
        let snapshot = store.snapshot().unwrap();
        assert_eq!(
            without_time(snapshot, &export),
            export,
            "{version}: the workspace does not hold what its export holds"
        );
        drop(store);

        let conn = rusqlite::Connection::open(&path).unwrap();
        let id: i32 = conn
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .unwrap();
        assert_eq!(id, APPLICATION_ID, "{version}: the workspace has no mark");
        drop(conn);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}

#[test]
fn the_export_of_each_release_imports_with_all_its_data() {
    for (version, dir) in fixtures() {
        let export = read_export(&dir);
        let store = Store::open_in_memory().unwrap();
        store
            .import(&export)
            .unwrap_or_else(|error| panic!("{version}: the export does not import: {error}"));
        assert_eq!(
            without_time(store.snapshot().unwrap(), &export),
            export,
            "{version}: the import lost data"
        );
    }
}
