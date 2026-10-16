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
    let config_path = hds_store::config::config_path()?;
    let mut config = hds_store::config::Config::load(&config_path)?;
    let (store, path) = first_workspace(&mut config)?;
    app::App::new(store, path, config, config_path)?.run(started)
}

/// Opens the workspace that the person opened last. When it does not open,
/// it goes off the list, and the workspace before it is next. When no recent
/// workspace opens, the workspace in the data directory opens. A path in
/// `HOTDOGSTAND_WORKSPACE` comes before all of them.
fn first_workspace(
    config: &mut hds_store::config::Config,
) -> anyhow::Result<(hds_store::Store, std::path::PathBuf)> {
    if !hds_store::workspace_is_given() {
        while let Some(path) = config.recent.first().cloned() {
            match hds_store::Store::open_existing(&path) {
                Ok(store) => return Ok((store, path)),
                Err(error) => {
                    eprintln!("hotdogstand: {}: {error}", path.display());
                    config.forget(&path);
                }
            }
        }
    }
    let path = hds_store::workspace_path()?;
    match hds_store::Store::open(&path) {
        Ok(store) => Ok((store, path)),
        Err(error) => {
            anyhow::bail!("Cannot open the workspace file {}: {error}", path.display());
        }
    }
}
