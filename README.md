<div align="center">

<img src="assets/hotdogstand.svg" alt="HotDogStand" width="180">

### A ticket manager for a small team, with the look of Windows 7

_One process, one SQLite file, no account with anybody_

<p align="center">
  <img src="https://img.shields.io/badge/Python-3.12%2B-3776AB?style=for-the-badge&logo=python&logoColor=white" alt="Python 3.12 or newer"/>
  <img src="https://img.shields.io/badge/status-planning-lightgrey?style=for-the-badge" alt="Status: planning"/>
</p>

<p align="center">
  <a href="https://ko-fi.com/stivengjekaj"><img src="https://img.shields.io/badge/Ko--fi-Support_this_project-FF5E5B?style=for-the-badge&logo=ko-fi&logoColor=white" alt="Support this project on Ko-fi"/></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="MIT License"/>
</p>

<p align="center">
  <a href="#overview"><b>Overview</b></a> |
  <a href="#features"><b>Features</b></a> |
  <a href="#quick-start"><b>Quick Start</b></a> |
  <a href="docs/architecture.md"><b>Architecture</b></a> |
  <a href="TODO.md"><b>Roadmap</b></a>
</p>

</div>

---

## Overview

**HotDogStand** keeps the tickets of a team: the bugs, the requests, and the
work that is not done. It runs on your own server or your own computer. The
data stays in one SQLite file, and nothing goes to another service.

It looks like a desktop from 2009. The tickets are in a list view in a window.
A double click opens a ticket in its own window, with a button on the
taskbar. You can open many tickets at the same time and move them on the
desktop.

The name comes from "Hot Dog Stand", the red and yellow color scheme of
Windows 3.1. HotDogStand ships it as a theme.

Nothing is built yet. [docs/architecture.md](docs/architecture.md) is the
plan, and [TODO.md](TODO.md) is the list of steps.

## Features

<table>
<tr>
<td width="50%" valign="top">

### Tickets

- Create, edit, assign, comment on, close, and reopen a ticket
- A title, a description in Markdown, a status, a priority, an assignee, and
  labels
- A history that shows who changed what, and when
- One workspace, with accounts that an admin makes

</td>
<td width="50%" valign="top">

### The desktop

- A list view that sorts by each column and filters by each field
- The sort and the filter are in the address, so you can send a view to a
  person
- A detail window for each ticket, with a button on the taskbar
- A full page for each window when the browser runs no script

</td>
</tr>
</table>

## Quick Start

This section shows the plan. The commands do not work yet.

You need [uv](https://docs.astral.sh/uv/).

```
git clone https://github.com/Stiven-Gjekaj/HotDogStand
cd HotDogStand
uv sync
uv run hotdogstand migrate
uv run hotdogstand create-user --admin
uv run hotdogstand serve
```

Then open http://127.0.0.1:8077 and log in.

## Documentation

| Document | What it holds |
| -------- | ------------- |
| [docs/architecture.md](docs/architecture.md) | The parts, the data model, the styling, and the reasons |
| [TODO.md](TODO.md) | What is not built yet |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to help |
| [SUPPORT.md](SUPPORT.md) | Where to ask a question |
| [SECURITY.md](SECURITY.md) | How to report a security problem |

## Not affiliated with Microsoft

HotDogStand copies a look. It is not a product of Microsoft, and Microsoft
does not support it. Windows is a trademark of Microsoft Corporation. The
project uses no icon, font, wallpaper, or sound from Microsoft.

## License

MIT. See [LICENSE](LICENSE) and [TERMS.md](TERMS.md).
