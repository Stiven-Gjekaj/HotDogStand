<div align="center">
  <a href="README.md"><img src="assets/hotdogstand.svg" alt="HotDogStand" height="44"></a>
</div>

# Contributing to HotDogStand

Thank you for your interest. HotDogStand is a ticket manager on your own
computer, with the look of Windows 7. Reports of a fault, corrections to the
documents, and new code are all welcome.

## What this project is, and what it is not

HotDogStand is a native desktop application that keeps its data in one
SQLite file on the computer that runs it. This is on purpose. A person must be
able to install it and work, with no account, no server, and no network.

A change that adds a server, a network connection, sync, or a webview goes the
wrong way. Live sharing is not planned. A person who wants to share tickets
sends an export.

The look is Windows 7. A change to the look must fit that look. A modern flat
control in the middle of an Aero window is a fault.

## Ways to help

- Report a fault or ask for a feature. Open an issue.
- Correct the documents in `docs/` or the readme.
- Take a task from [TODO.md](TODO.md).

Open an issue before you start a large piece of work, so that we agree on the
way to do it before you write it.

## Development setup

You need `rustup`. The repository pins the toolchain in
`rust-toolchain.toml`, and `rustup` installs it the first time you build.

    git clone https://github.com/Stiven-Gjekaj/HotDogStand
    cd HotDogStand
    cargo run

On Linux, Slint needs the development files of fontconfig and xkbcommon:

    sudo apt install libfontconfig-dev libxkbcommon-dev

Run the tests:

    cargo test

Turn on the hook that checks the format before each commit:

    git config core.hooksPath .githooks

To work on a copy of the data that is not your real workspace, set
`HOTDOGSTAND_WORKSPACE` to the path of another file.

## Where a change lives

| Change | Files |
| ------ | ----- |
| A rule about a ticket | `crates/hds-core/src/ticket.rs` and its tests |
| A new sort or filter | `crates/hds-core/src/query.rs` |
| An export format | `crates/hds-core/src/export.rs` |
| A new column or table | a new step in `crates/hds-store/src/migrations.rs` |
| A control | `crates/hotdogstand/ui/aero/` |
| A window | `crates/hotdogstand/ui/` and `crates/hotdogstand/src/windows/` |
| A color or a size | `crates/hotdogstand/ui/theme.slint` |
| The logo or the icon of the application | `assets/hotdogstand.svg`, then run `scripts/make-icons.sh` and commit what it writes |
| The macOS bundle | `packaging/macos/Info.plist` and `scripts/bundle-macos.sh` |
| The Linux desktop entry | `packaging/linux/hotdogstand.desktop` and `scripts/install-linux.sh` |

## Rules that this project holds to

- **Code and its tests go in one commit. Documents go in their own.**
- **A commit carries no version prefix and changes no version.** The version in
  `Cargo.toml` moves only when something is released.
- **Each change to the schema is a new migration step.** Do not change a step
  that is in a release. Add a new one. A workspace file from an old release
  must open in a new one.
- **A rule lives in `hds-core`.** It knows nothing about SQLite or Slint. A
  test of a rule opens no file and no window.
- **A change and its event go in one transaction.** The history must agree
  with the data.
- **A change to the export format raises its version.** An old export must
  stay readable.
- **An error that a person can correct says what to change.** It names the
  field or the file. An error that a person cannot correct keeps its cause.
- **No `unwrap` outside the tests.** A fault in the data must not close the
  application and lose the work of the person.
- **A color or a size goes in `theme.slint`.** A control that holds its own
  color breaks the second theme.
- **Run it. Do not conclude that it works.** Say so plainly when a measurement
  does not support the conclusion.

## Before you open a pull request

Run these, exactly as the workflow does:

    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

Add tests for what you change. A test builds the people, the tickets, and the
labels that it needs in a database in memory. It does not read them from a
file that a person edits.

For a change to the look, add a screenshot to the pull request, in the Aero
theme and in the Hot Dog Stand theme.

## Style

- Match the code around you. Small functions and clear names.
- Add a dependency only with a reason. Say the reason in the pull request.
- Write documents and comments in ASD-STE100 Simplified Technical English.
  Use short sentences, the active voice, and the present tense.
- Use no em-dash and no emoji in source, documents, commit messages, or
  examples.

## Commit messages and pull requests

- Write a present-tense subject that describes the change.
- Keep one logical change in one commit.
- In the pull request, say what changed, why, and how you tested it.

An agent that works in this repository follows [AGENTS.md](AGENTS.md).

## Making a release

1. Move the notes under "Unreleased" in CHANGELOG.md to a new section with
   the version and the date, such as `## 0.2.0 (2026-11-01)`.
2. Set the version in the `[workspace.package]` table of `Cargo.toml`, and
   run `cargo build` so that `Cargo.lock` follows.
3. Commit the two changes, one commit each, and push them.
4. Tag the commit with `v` and the version, such as `v0.2.0`, and push the
   tag. The release workflow builds the packages and makes the release, with
   the section of CHANGELOG.md as its notes.

## Reporting a security problem

Do not open a public issue. See [SECURITY.md](SECURITY.md).

## Code of conduct

By taking part you agree to the [Code of Conduct](CODE_OF_CONDUCT.md).
