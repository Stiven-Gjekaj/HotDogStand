<div align="center">
  <a href="../README.md"><img src="../assets/hotdogstand.svg" alt="HotDogStand" height="44"></a>
</div>

# Architecture

This document is the plan for the first version. It says what the parts are,
where they live, and why. No code exists yet. Change this document before you
change the plan.

## The shape of the program

HotDogStand is a native desktop application. It is one binary. It keeps its
data in one SQLite file on the computer that runs it. It does not start a
server, it does not open a network connection, and it does not use a webview.

Slint draws each window with its own renderer. The application draws the look
of Windows 7 itself, so the look is the same on Windows, macOS, and Linux.

The data does not leave the computer, except when the person exports it.
There is no live sharing and no sync.

Reason: a person must be able to install the application, open it, and work.
No account, no server, and no network means that nothing can stop this.

## Tech stack

| Part | Choice | Reason |
| ---- | ------ | ------ |
| Language | Rust, edition 2024, toolchain pinned in `rust-toolchain.toml` | One fast binary with no runtime to install. |
| UI | Slint 1.x | It draws the UI itself. No webview, no browser engine. The `.slint` language is made for a set of custom controls. |
| Database | SQLite through `rusqlite`, with the `bundled` feature | One file. The binary carries SQLite, so the computer needs no copy of it. |
| Migrations | `rusqlite_migration` | Each change to the schema is a step in the repository. |
| Data directory | `directories` | It finds the correct directory for the data on each operating system. |
| Export | `serde`, `serde_json`, `csv` | JSON for a full copy. CSV for a spreadsheet. |
| Markdown | `pulldown-cmark` | The description and the comments use Markdown. |
| Time | `jiff` | Timestamps in UTC, shown in the local time zone. |
| Errors | `thiserror` in the libraries, `anyhow` in the binary | The same as the other projects of the maintainer. |
| Lint and format | `cargo fmt`, `cargo clippy` | |
| Tests | `cargo test` | |

## Project structure

    HotDogStand/
      AGENTS.md            rules for an agent
      README.md            what it is and how to start it
      CONTRIBUTING.md      how to help
      CHANGELOG.md         what each release changes
      TODO.md              what is not built yet
      LICENSE              MIT
      Cargo.toml           the workspace
      rust-toolchain.toml
      assets/              the logo and the screenshots for the readme
      docs/
        architecture.md    this document
      crates/
        hds-core/          the rules, with no I/O
          src/
            ticket.rs      the ticket, its status, its priority
            event.rs       the history of a ticket
            query.rs       sort and filter
            export.rs      JSON and CSV, written to any `io::Write`
            error.rs
        hds-store/         SQLite
          src/
            lib.rs         open a workspace file
            migrations.rs
            tickets.rs     read and write tickets, comments, events
            labels.rs
            people.rs
        hotdogstand/       the application, the binary
          src/
            main.rs
            windows/       one Rust module for each window
          ui/
            aero/          the controls: frame, button, list view, tabs, fields
            theme.slint    the colors and the sizes, as one global
            main.slint     the ticket list window
            ticket.slint   the ticket detail window
            people.slint   the people window
            labels.slint   the labels window
            about.slint    the about window, which credits Slint
          fonts/           Selawik, with its licence
          icons/
          build.rs         compiles the `.slint` files
      .github/
        workflows/ci.yml
        ISSUE_TEMPLATE/

`hds-core` holds the rules and knows nothing about SQLite or Slint. A test of
a rule calls `hds-core` and opens no file. `hds-store` changes the data, and
writes each change and its event in one transaction. The application reads
the input of the person, calls the store, and shows the result.

## Data model

One workspace is one SQLite file. The first version opens one workspace, in
the data directory of the operating system:

| System | Path |
| ------ | ---- |
| Windows | `%APPDATA%\HotDogStand\workspace.db` |
| macOS | `~/Library/Application Support/HotDogStand/workspace.db` |
| Linux | `~/.local/share/hotdogstand/workspace.db` |

The variable `HOTDOGSTAND_WORKSPACE` gives a different path. A contributor
uses it to work on a copy, and a test uses it to keep away from the real file.

### person

One person uses the application. The people are the names that a ticket can
be assigned to. They are not accounts, and they have no password.

| Column | Type | Note |
| ------ | ---- | ---- |
| id | integer | primary key |
| name | text | unique, not empty |
| is_active | integer | 0 or 1. A person who leaves is hidden, not deleted. |
| created_at | text | UTC, RFC 3339 |

An inactive person does not show in the list of assignees. Their tickets keep
their name.

### ticket

| Column | Type | Note |
| ------ | ---- | ---- |
| id | integer | primary key. The number that people say: "ticket 42". |
| title | text | not empty, 200 characters or fewer |
| description | text | Markdown |
| status | text | `open`, `in_progress`, or `closed` |
| priority | text | `low`, `normal`, `high`, or `urgent` |
| assignee_id | integer | references person, can be empty |
| created_at | text | |
| updated_at | text | |
| closed_at | text | empty when the ticket is not closed |

A CHECK constraint holds the values of `status` and `priority`. "Close" sets
`status` to `closed` and sets `closed_at`. "Reopen" clears `closed_at`.

### label and ticket_label

| Column | Type | Note |
| ------ | ---- | ---- |
| label.id | integer | primary key |
| label.name | text | unique |
| label.color | text | a hex color, for example `#3a6ea5` |
| ticket_label.ticket_id | integer | references ticket |
| ticket_label.label_id | integer | references label |

The primary key of `ticket_label` is the two columns together.

### comment

| Column | Type | Note |
| ------ | ---- | ---- |
| id | integer | primary key |
| ticket_id | integer | references ticket |
| body | text | Markdown, not empty |
| created_at | text | |
| edited_at | text | empty when nobody edited it |

### event

| Column | Type | Note |
| ------ | ---- | ---- |
| id | integer | primary key |
| ticket_id | integer | references ticket |
| kind | text | `created`, `edited`, `assigned`, `status`, `priority`, `labeled`, `unlabeled` |
| old_value | text | can be empty |
| new_value | text | can be empty |
| created_at | text | |

The store writes an event in the same transaction as the change. The detail
window shows the events and the comments in one line of time.

Reason: "when did this close, and what changed" is the first question about a
ticket.

### setting

| Column | Type | Note |
| ------ | ---- | ---- |
| key | text | primary key |
| value | text | |

The first keys are `workspace_name` and `theme`.

## Export

"File > Export" writes a copy of the workspace to a file that the person
chooses. The export reads the data in one transaction, so the file agrees with
itself.

| Format | Holds | For |
| ------ | ----- | --- |
| JSON | every ticket, comment, event, label, person, and setting | a backup, a move to another computer, another tool |
| CSV | one row for each ticket, with the columns of the list view | a spreadsheet |

The JSON file starts with `"format": "hotdogstand"` and `"version": 1`. A
change to its shape raises the version. The CSV export takes the sort and the
filter of the list view, so the file holds what the person sees.

Import of the JSON file comes after the first version. The format is made so
that an import can read it without loss.

## The windows

- **The ticket list** is the main window. It has a menu bar, a toolbar, the
  list view, and a status bar with the number of tickets.
- A click on a column header sorts by that column. A second click turns the
  order around.
- The toolbar filters by status, priority, assignee, and label, and has a
  search field for the title.
- A double click or Enter on a row opens the ticket in its own window. More
  than one ticket can be open at the same time. Each one is a real window of
  the operating system, with its own button on the taskbar of the system.
- **The ticket window** has the tabs "General", for the fields, and
  "History", for the comments and the events.
- **People** and **Labels** are small windows from the "Edit" menu.
- **About** names the version, the licence, and Slint.

## Styling

- The project draws its own set of Aero controls in `ui/aero/`: the window
  frame, the title bar and its buttons, the push button, the list view with
  column headers, the tabs, the text field, the combo box, the menu, and the
  status bar.
- Each window has no frame from the system. `ui/aero/frame.slint` draws the
  glass frame and the title bar, and the Rust code asks the system to move or
  resize the window when the person drags it. A setting turns this off and
  uses the frame of the system, for a desktop where a frameless window does
  not behave well.
- `theme.slint` holds every color and size as one global. A theme changes the
  global and nothing else. The first theme is "Aero". The second is "Hot Dog
  Stand", the red and yellow scheme of Windows 3.1, which gives the project
  its name.
- The font is Selawik, which Microsoft publishes under the SIL Open Font
  Licence as an open replacement for Segoe UI. The project ships it with its
  licence. It does not ship Segoe UI, because the licence of Segoe UI does not
  permit that.
- The project draws its own icons, or takes them from a set with an open
  licence. It uses no icon, wallpaper, logo, or sound from Windows.
- Keyboard use works: Tab moves between the controls, Enter opens a ticket,
  Escape closes a ticket window, and the menus have access keys.

## Slint and the licence

The code of HotDogStand is MIT. Slint is available under the GPLv3, under the
Slint Royalty-free Licence, and under a paid licence. HotDogStand uses the
Royalty-free Licence. That licence permits a desktop application at no cost,
and asks for credit to Slint. The About window gives that credit.

A binary of HotDogStand therefore contains code under two licences. TERMS.md
says this to the person who uses it.

## Tests

- A test of a rule calls `hds-core` and opens no file.
- A test of the store opens an SQLite database in memory and builds the
  people, the tickets, and the labels that it needs.
- A test of the migrations runs every step on an empty database, and checks
  the tables that come out.
- A test of the export writes to a `Vec<u8>`, reads the result back, and
  compares it with the data that went in.
- The CI runs `cargo fmt --check`, `cargo clippy -- -D warnings`, and
  `cargo test` on Linux, macOS, and Windows.

## Not in the first version

- Import of an export file.
- More than one workspace file.
- A password or encryption for the workspace file.
- Attachments.
- Live sharing, sync, or a server. These are not planned.

[TODO.md](../TODO.md) holds these, with the reason for each.
