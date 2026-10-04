<div align="center">
  <a href="README.md"><img src="assets/hotdogstand.svg" alt="HotDogStand" height="44"></a>
</div>

# Contributing to HotDogStand

Thank you for your interest. HotDogStand is a ticket manager for a small team,
with the look of Windows 7. Reports of a fault, corrections to the documents,
and new code are all welcome.

## What this project is, and what it is not

HotDogStand is for one team in one workspace. It runs as one process with one
SQLite file. This is on purpose. A small team must be able to install it in a
few minutes and back it up with a copy of one file.

A change that adds a second service, such as a queue, a cache server, or a
JavaScript build, goes the wrong way. A pull request that does it needs a
reason.

The look is Windows 7. A change to the look must fit that look. A modern flat
control in the middle of a 7.css window is a fault.

## Ways to help

- Report a fault or ask for a feature. Open an issue.
- Correct the documents in `docs/` or the readme.
- Take a task from [TODO.md](TODO.md).

Open an issue before you start a large piece of work, so that we agree on the
way to do it before you write it.

## Development setup

You need `uv`. Then:

    git clone https://github.com/Stiven-Gjekaj/HotDogStand
    cd HotDogStand
    uv sync

Run the tests:

    uv run pytest

Start the server with a database for development:

    uv run hotdogstand migrate
    uv run hotdogstand create-user --admin
    uv run hotdogstand serve --reload

## Where a change lives

| Change | Files |
| ------ | ----- |
| A rule about a ticket | `src/hotdogstand/tickets/service.py` and its tests |
| A new sort or filter | `src/hotdogstand/tickets/query.py` |
| A new column or table | `src/hotdogstand/db/models.py` and a new migration in `db/migrations/` |
| A window | `src/hotdogstand/web/templates/windows/` |
| The look of the desktop | `src/hotdogstand/web/static/css/app.css` |
| A theme | the custom properties at the top of `app.css` |
| A new command | `src/hotdogstand/cli.py` and `tests/test_cli.py` |

Do not edit `7.css`. It is a copy of a project of another person. A fix to it
goes to that project first.

## Rules that this project holds to

- **Code and its tests go in one commit. Documents go in their own.**
- **A commit carries no version prefix and changes no version.** The version in
  `pyproject.toml` moves only when something is released.
- **Each change to the schema is a migration.** Do not change a migration that
  is in a release. Add a new one.
- **A rule lives in a service, not in a route.** A route reads the request and
  returns a page. A test of a rule does not start a server.
- **An error that a person can correct is a `HotDogStandError`.** It says what
  to change. An error that a person cannot correct stays an ordinary exception
  and keeps its traceback.
- **A setting that nothing reads is an error.** A misspelled variable that the
  server ignores lies about what the server does.
- **Each page works with no script.** The script makes the desktop better. It
  does not hold a feature.
- **Run it. Do not conclude that it works.** Say so plainly when a measurement
  does not support the conclusion.

## Before you open a pull request

Run these, exactly as the workflow does:

    uv run ruff format --check .
    uv run ruff check .
    uv run pytest

Add tests for what you change. A test builds the users, the tickets, and the
labels that it needs. It does not read them from a file that a person edits.

For a change to the look, add a screenshot to the pull request.

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

## Reporting a security problem

Do not open a public issue. See [SECURITY.md](SECURITY.md).

## Code of conduct

By taking part you agree to the [Code of Conduct](CODE_OF_CONDUCT.md).
