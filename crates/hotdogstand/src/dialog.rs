//! The dialogs of the system: the file dialogs and the warnings.
//!
//! The smoke test cannot click in a dialog of the system. A build with the
//! `smoke` feature reads the answers from the file that [`ANSWERS_VAR`]
//! names, one answer on each line, in the order that the dialogs open. The
//! test adds a line before each step, so the file is read again for each
//! dialog. For a file dialog, the answer is a path. For a warning, the answer
//! is `ok` or `cancel`. An empty line cancels the dialog. A build without the
//! feature ignores the variable.

use std::path::PathBuf;

/// The variable that names the file of answers of a smoke test.
#[cfg_attr(not(feature = "smoke"), allow(dead_code))]
pub const ANSWERS_VAR: &str = "HOTDOGSTAND_SMOKE_ANSWERS";

pub fn pick_file(dialog: rfd::FileDialog) -> Option<PathBuf> {
    match next_answer() {
        Some(answer) => path(answer),
        None => dialog.pick_file(),
    }
}

pub fn save_file(dialog: rfd::FileDialog) -> Option<PathBuf> {
    match next_answer() {
        Some(answer) => path(answer),
        None => dialog.save_file(),
    }
}

/// Shows a warning with the buttons OK and Cancel. True for OK.
pub fn confirm(dialog: rfd::MessageDialog) -> bool {
    match next_answer() {
        Some(answer) => answer == "ok",
        None => {
            dialog.set_buttons(rfd::MessageButtons::OkCancel).show() == rfd::MessageDialogResult::Ok
        }
    }
}

#[cfg_attr(not(feature = "smoke"), allow(dead_code))]
fn path(answer: String) -> Option<PathBuf> {
    (!answer.is_empty()).then(|| PathBuf::from(answer))
}

#[cfg(not(feature = "smoke"))]
fn next_answer() -> Option<String> {
    None
}

/// Gives the next answer, or `None` when the variable is not set. When the
/// test gave too few answers, the application stops, because a dialog of
/// the system would wait for ever in a headless run.
#[cfg(feature = "smoke")]
fn next_answer() -> Option<String> {
    use std::cell::Cell;
    thread_local!(static NEXT: Cell<usize> = const { Cell::new(0) });
    let file = std::env::var_os(ANSWERS_VAR)?;
    let answers = std::fs::read_to_string(file).unwrap_or_default();
    let index = NEXT.with(|next| next.replace(next.get() + 1));
    match answers.lines().nth(index) {
        Some(answer) => Some(answer.trim().to_owned()),
        None => {
            eprintln!("hotdogstand: {ANSWERS_VAR} has no answer for dialog {index}");
            std::process::exit(3);
        }
    }
}
