//! Writes the JSON export of a workspace file. scripts/make-fixtures.sh copies
//! this file into the examples of a released version, so the export comes
//! from the code of that version.
//!
//!     cargo run -p hds-store --example fixture_export -- <workspace> <json>

use std::fs::File;
use std::io::BufWriter;

use hds_core::export::write_json;
use hds_store::Store;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let workspace = args.next().ok_or("give the path of the workspace file")?;
    let json = args.next().ok_or("give the path of the JSON file")?;
    let store = Store::open(workspace.as_ref())?;
    let snapshot = store.snapshot()?;
    write_json(&snapshot, BufWriter::new(File::create(json)?))?;
    Ok(())
}
