<div align="center">
  <a href="README.md"><img src="assets/hotdogstand.svg" alt="HotDogStand" height="44"></a>
</div>

# What is left to build

A task with a box is not built. A task without one is done.

## Where things are

The application opens, keeps tickets in the workspace file, and shows them in
the Aero windows. The plan is in [docs/architecture.md](docs/architecture.md).
The steps below are in the order to build them. Each step is one or more
commits, and each commit does one thing.

A step marked "built, not tried" has code but nobody used it by hand yet.

## The first version

### The skeleton

- The workspace `Cargo.toml`, `rust-toolchain.toml`, and the three crates
- A Slint window that opens and closes
- A CI workflow that runs `cargo fmt --check`, `cargo clippy`, and
  `cargo test` on Linux, macOS, and Windows

### The rules

- The ticket, its status, and its priority in `hds-core`
- The events, and the rule that makes an event from each change
- Sort and filter in `hds-core::query`

### The store

- Open the workspace file in the data directory, and make it when it is
  not there
- The first migration: person, ticket, label, ticket_label, comment,
  event, setting
- A test that runs every migration on an empty database
- Create a ticket, with an event
- Edit the title, the description, and the priority, with an event for
  each change
- Assign and unassign
- Close and reopen
- Add and remove a label
- Add a comment
- Edit a comment in the History tab
- Add, rename, hide, and show a person

### The controls

- `theme.slint` with the Aero theme
- Selawik, with its licence
- The window frame and the title bar, with move and resize
- The setting that uses the frame of the system
- Push button, text field, multi-line text field, combo box
- List view with column headers that sort
- Tabs, command bar, status bar, scroll bar, check box, menu

### The windows

- The ticket list window, with the row of filters
- The ticket window, with the tabs "General" and "History"
- Markdown in the description and the comments, with the `StyledText` of
  Slint
- More than one ticket window open at the same time, each one a step below
  the one before it
- The People window and the Labels window
- The About window, with the credit to Slint
- Keyboard: Tab, the arrow keys in the list, Enter, and Escape
- Access keys: Alt and the first letter of each command of the command bar
- Shortcuts: Ctrl+N or Command+N for a new ticket, Ctrl+F or Command+F for
  the search, and Ctrl+S, Command+S, Ctrl+Enter, or Command+Enter in a ticket
  window
- Accessible roles for the rows, the headers, the tabs, and the controls
- The "Hot Dog Stand" theme
- A dark form of each theme
- The menus in the menu bar of macOS, with a check mark on each setting
- The icon of the application on each system: in the .exe file on Windows,
  in the app bundle on macOS, and in the desktop entry on Linux
- Icons inside the windows, for the command bar and the menus

### Export

- Export to JSON, with `format` and `version` at the top
- Export to CSV, with the sort and the filter of the list view
- "Export" with the file dialog of the system, tried on macOS
- A test that exports, reads the file back, and compares

### The release

- Screenshots in the readme, of the application with the demo workspace
- Measure the time to open the application, and the time to sort and filter
  10,000 tickets. The numbers are below.
- [ ] Build a binary for Windows, macOS, and Linux in the CI
- [ ] Tag `v0.1.0`

## Measurements

Measured on a Mac with an Apple M5 processor, with a release build and a
workspace of 10,000 tickets that `examples/measure.rs` makes. Run it again
with:

    cargo run --release -p hds-store --example measure -- big.db
    HOTDOGSTAND_TIMING=1 HOTDOGSTAND_WORKSPACE=big.db cargo run --release

| What | Time |
| ---- | ---- |
| Open the file and read 10,000 tickets | 4.2 ms |
| Start the application to its first frame | 146 to 195 ms, three runs |
| Show the list after a change of the filter, in the application | 0.1 to 4.2 ms |
| Sort 8,000 tickets by priority, in the query alone | 0.35 ms |
| Sort 8,000 tickets by assignee, in the query alone | 2.8 ms |
| Search the titles of 10,000 tickets for a word | 0.27 ms |

## Known problems

- [ ] On macOS, the menus of the menu bar show only while the main window
      has the focus. A ticket window, the People window, and the Labels
      window have no menu bar of their own.
- [ ] Selawik has no italic face, so Markdown in italics shows upright.
- [ ] A link in Markdown shows as a link, but a click on it does nothing.

## After the first version

Each of these waits for a person who needs it. A feature that nobody uses rots.

- [ ] Import of a JSON export. The format is ready for it. The import must
      refuse a file with a version that it does not know.
- [ ] More than one workspace file, with "File > Open".
- [ ] Attachments. This needs a directory beside the workspace file, a limit
      on size, and a place in the export.
- Packages for Homebrew, Scoop, and winget. The winget package waits for the
  review of microsoft/winget-pkgs#446597.

Live sharing, sync, a server, and a password are not planned. HotDogStand is a
local application. A team that wants to share the tickets sends an export.
