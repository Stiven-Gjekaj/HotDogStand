<div align="center">
  <a href="README.md"><img src="assets/hotdogstand.svg" alt="HotDogStand" height="44"></a>
</div>

# Changelog

Every release is written here, newest first. A version is `MAJOR.MINOR.PATCH`.
A change to the schema of the workspace file, or to the export format, raises
the minor number while the major number is 0. A new release always opens a
workspace file from an older one.

## Unreleased

Nothing yet.

## 0.1.0 (2026-10-04)

The first release. HotDogStand keeps the tickets of one workspace in one
SQLite file on your computer, and shows them in windows with the look of
Windows 7.

### Tickets

- Create, edit, assign, comment on, close, and reopen a ticket.
- A title, a description, a status, a priority, an assignee, and labels.
- The description and the comments use Markdown.
- Edit a comment after you add it.
- A history that records each change, with its time.
- People to assign tickets to, and labels with a color. A person who leaves
  is hidden, not deleted.

### The windows

- A list that sorts by each column and filters by status, priority,
  assignee, label, and words in the title.
- A window for each ticket, with the tabs General and History. More than one
  can be open.
- A glass frame drawn by the application, or the frame of the system.
- Two themes, Aero and Hot Dog Stand, each with a dark form.
- The Selawik font, in the binary.
- Access keys, keyboard shortcuts, and accessible roles for screen readers.
- On macOS, the menus are in the menu bar of the system.

### Your data

- Export the full workspace to JSON, or the list as you see it to CSV.
- The workspace file opens in each later version.

### Packages

- Windows: a zip file with `HotDogStand.exe`, which has its icon.
- macOS: `HotDogStand.app`, for Apple silicon and for Intel.
- Linux: an archive with the program, its icons, its desktop entry, and an
  install script. Built on Ubuntu 22.04.

### Known problems

- The packages are not signed. Windows SmartScreen and macOS Gatekeeper warn
  before the first start. The readme says how to open the application.
- On macOS, the menus of the menu bar show only while the main window has the
  focus.
- Selawik has no italic face, so Markdown in italics shows upright.
- A click on a link in Markdown does nothing.
