//! HotDogStand: a ticket manager on your own computer, with the look of
//! Windows 7.

mod app;
mod format;
mod frame;

/// The code that Slint generates from the `.slint` files. It does not follow
/// the lints of this project, so they stop at this module.
mod ui {
    #![allow(clippy::all, clippy::unwrap_used, clippy::expect_used)]
    slint::include_modules!();
}

fn main() -> anyhow::Result<()> {
    let path = hds_store::workspace_path()?;
    let store = match hds_store::Store::open(&path) {
        Ok(store) => store,
        Err(error) => {
            anyhow::bail!("Cannot open the workspace file {}: {error}", path.display());
        }
    };
    app::App::new(store, path)?.run()
}
