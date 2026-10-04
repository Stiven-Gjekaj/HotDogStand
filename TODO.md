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
- [ ] Edit a comment in the History tab. The store can do it. The window
      cannot yet.
- Add, rename, hide, and show a person

### The controls

- `theme.slint` with the Aero theme
- [ ] Selawik, with its licence
- The window frame and the title bar, with move and resize
- The setting that uses the frame of the system
- Push button, text field, multi-line text field, combo box
- List view with column headers that sort
- Tabs, command bar, status bar, scroll bar, check box, menu

### The windows

- The ticket list window, with the row of filters
- The ticket window, with the tabs "General" and "History"
- [ ] Markdown in the description and the comments
- [ ] More than one ticket window open at the same time: built, not tried
- The People window and the Labels window
- The About window, with the credit to Slint
- Keyboard: Tab, the arrow keys in the list, Enter, and Escape
- [ ] Access keys in the command bar and the Options menu
- [ ] The tabs as accessible elements, so a screen reader can change them
- The "Hot Dog Stand" theme
- A dark form of each theme
- The menus in the menu bar of macOS, with a check mark on each setting
- [ ] The icons, and the icon of the application on each system

### Export

- Export to JSON, with `format` and `version` at the top
- Export to CSV, with the sort and the filter of the list view
- [ ] "Export" with the file dialog of the system: built, not tried
- A test that exports, reads the file back, and compares

### The release

- [ ] A screenshot in the readme
- [ ] Measure the time to open the application, and the time to sort and
      filter 10,000 tickets, and write the numbers here
- [ ] Build a binary for Windows, macOS, and Linux in the CI
- [ ] Tag `v0.1.0`

## After the first version

Each of these waits for a person who needs it. A feature that nobody uses rots.

- [ ] Import of a JSON export. The format is ready for it. The import must
      refuse a file with a version that it does not know.
- [ ] More than one workspace file, with "File > Open".
- [ ] Attachments. This needs a directory beside the workspace file, a limit
      on size, and a place in the export.
- [ ] Packages for Homebrew, Scoop, and winget.

Live sharing, sync, a server, and a password are not planned. HotDogStand is a
local application. A team that wants to share the tickets sends an export.
