//! HotDogStand: a ticket manager on your own computer, with the look of
//! Windows 7.

// A release build on Windows opens no console window next to the
// application.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod format;
mod frame;

/// The code that Slint generates from the `.slint` files. It does not follow
/// the lints of this project, so they stop at this module.
mod ui {
    #![allow(clippy::all, clippy::unwrap_used, clippy::expect_used)]
    slint::include_modules!();
}

/// The name that a Linux desktop uses to match the windows to
/// `hotdogstand.desktop` and to its icon.
const APP_ID: &str = "hotdogstand";

fn main() -> anyhow::Result<()> {
    let started = std::time::Instant::now();
    // The id only matches the windows to their icon, so a backend that does
    // not take it, such as the headless one, does not stop the application.
    if cfg!(target_os = "linux")
        && let Err(error) = slint::set_xdg_app_id(APP_ID)
    {
        eprintln!("hotdogstand: the desktop id is not set: {error}");
    }
    let path = hds_store::workspace_path()?;
    let store = match hds_store::Store::open(&path) {
        Ok(store) => store,
        Err(error) => {
            anyhow::bail!("Cannot open the workspace file {}: {error}", path.display());
        }
    };
    app::App::new(store, path)?.run(started)
}
