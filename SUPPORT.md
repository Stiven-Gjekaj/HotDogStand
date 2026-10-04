<div align="center">
  <a href="README.md"><img src="assets/hotdogstand.svg" alt="HotDogStand" height="44"></a>
</div>

# Getting help

## Read first

- [README.md](README.md) says what HotDogStand is and how to start it.
- [docs/architecture.md](docs/architecture.md) explains the parts, the data
  model, where the workspace file is, and the export formats.
- [TODO.md](TODO.md) says what is not built yet. Look here before you report
  that something is missing.

## Where your data is

The workspace is one file. The path is different on each system:

| System | Path |
| ------ | ---- |
| Windows | `%APPDATA%\HotDogStand\workspace.db` |
| macOS | `~/Library/Application Support/HotDogStand/workspace.db` |
| Linux | `~/.local/share/hotdogstand/workspace.db` |

Copy this file to make a backup. Close the application before you copy it.

## When the application does not open

- **A workspace file from a newer version.** An old version cannot open a
  file that a newer version changed. Install the newer version.
- **A damaged workspace file.** The message names the file. Put a backup in
  its place, or move the file away to start with an empty workspace.

## When the window frame behaves badly

Some Linux desktops do not move or resize a window with no frame from the
system. Turn on "Use the system frame" in "View > Options".

## Ask a question or report a fault

- Look through the
  [issues](https://github.com/Stiven-Gjekaj/HotDogStand/issues) first.
- Open a bug report for a fault, or a feature request for something new.

Say which version or commit you used, which system you used, what you did, and
what happened. A screenshot helps for a fault in the look. Do not attach your
workspace file. It holds your tickets. Make a small workspace that shows the
fault, and attach its export.

Do not use the issue tracker for a security problem. See
[SECURITY.md](SECURITY.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
