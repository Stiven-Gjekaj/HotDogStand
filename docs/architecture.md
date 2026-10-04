<div align="center">
  <a href="../README.md"><img src="../assets/hotdogstand.svg" alt="HotDogStand" height="44"></a>
</div>

# Architecture

This document is the plan for the first version. It says what the parts are,
where they live, and why. No code exists yet. Change this document before you
change the plan.

## The shape of the program

HotDogStand is one Python process. It serves HTML pages, keeps its data in one
SQLite file, and needs no other service. One command starts it:

    hotdogstand serve

The server makes each page. The browser gets HTML, and a small script moves the
windows on the desktop. A page also works with no script: each window has its
own address, and the browser opens it as a full page.

Reason: a ticket manager for a small team must be easy to install. One
process and one file are easy to copy, to back up, and to move.

## Tech stack

| Part | Choice | Reason |
| ---- | ------ | ------ |
| Language | Python 3.12 | The same as the other projects of the maintainer. |
| Package manager | uv | One command installs the project and its tools. |
| Web framework | FastAPI on Uvicorn | It does forms, cookies, and a JSON API later, with types. |
| Templates | Jinja2 | The server makes the HTML. |
| Partial updates | htmx | A sort or a filter replaces the list, not the full page. No build step. |
| Database | SQLite | One file. No server to run. |
| Database access | SQLAlchemy 2 | Typed models. A move to PostgreSQL later is possible. |
| Migrations | Alembic | Each change to the schema is a file in the repository. |
| Passwords | argon2-cffi | Argon2id is the current recommendation. |
| Markdown | markdown-it-py and nh3 | The description and the comments use Markdown. nh3 removes unsafe HTML. |
| Styling | 7.css, vendored | It draws the controls of Windows 7 in CSS. MIT licence. |
| Tests | pytest | The same as the other projects. |
| Lint and format | ruff | The same as the other projects. |

There is no Node.js and no JavaScript build. The script and the CSS are
static files in the repository.

## Project structure

    HotDogStand/
      AGENTS.md            rules for an agent
      README.md            what it is and how to start it
      CONTRIBUTING.md      how to help
      CHANGELOG.md         what each release changes
      TODO.md              what is not built yet
      LICENSE              MIT
      pyproject.toml
      assets/              the logo and the screenshots for the readme
      docs/
        architecture.md    this document
      src/hotdogstand/
        __init__.py
        __main__.py
        cli.py             serve, migrate, create-user
        app.py             makes the FastAPI application
        config.py          reads the settings from the environment
        errors.py          HotDogStandError, for an error a person can correct
        db/
          models.py        the tables
          session.py       the engine and the session
          migrations/      Alembic
        auth/
          passwords.py     hash and verify
          sessions.py      log in, log out, the current user
          csrf.py          the token on each form
          routes.py
        tickets/
          service.py       create, edit, assign, close, reopen
          query.py         sort and filter
          routes.py
        comments/
          service.py
          routes.py
        labels/
          service.py
          routes.py
        web/
          templates/
            base.html      the desktop, the taskbar, the start menu
            windows/       one template for each window
            partials/      the pieces that htmx replaces
          static/
            css/7.css      vendored, with its licence beside it
            css/app.css    the desktop and the layout
            js/windows.js  move, focus, minimize, and close a window
            icons/
      tests/
      .github/
        workflows/ci.yml
        ISSUE_TEMPLATE/

Each feature has a `service.py` and a `routes.py`. The service holds the
rules and knows nothing about HTTP. The routes read the request, call the
service, and return a page. A test of a rule calls the service and does not
start a server.

## Data model

One workspace means no workspace table. The name of the workspace is a
setting.

### user

| Column | Type | Note |
| ------ | ---- | ---- |
| id | integer | primary key |
| username | text | unique, lower case |
| display_name | text | |
| password_hash | text | Argon2id |
| role | text | `admin` or `member` |
| is_active | boolean | a person who leaves is deactivated, not deleted |
| created_at | timestamp | UTC |

A deactivated user cannot log in. Their tickets and comments keep their name.

### session

| Column | Type | Note |
| ------ | ---- | ---- |
| token_hash | text | primary key. The cookie holds the token. The database holds its SHA-256. |
| user_id | integer | references user |
| created_at | timestamp | |
| expires_at | timestamp | |

Reason: a copy of the database does not give a person a session.

### ticket

| Column | Type | Note |
| ------ | ---- | ---- |
| id | integer | primary key. The number that people say: "ticket 42". |
| title | text | not empty, 200 characters or fewer |
| description | text | Markdown |
| status | text | `open`, `in_progress`, or `closed` |
| priority | text | `low`, `normal`, `high`, or `urgent` |
| reporter_id | integer | references user |
| assignee_id | integer | references user, can be empty |
| created_at | timestamp | |
| updated_at | timestamp | |
| closed_at | timestamp | empty when the ticket is not closed |

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
| author_id | integer | references user |
| body | text | Markdown, not empty |
| created_at | timestamp | |
| edited_at | timestamp | empty when nobody edited it |

### event

| Column | Type | Note |
| ------ | ---- | ---- |
| id | integer | primary key |
| ticket_id | integer | references ticket |
| actor_id | integer | references user |
| kind | text | `created`, `edited`, `assigned`, `status`, `priority`, `labeled`, `unlabeled` |
| old_value | text | can be empty |
| new_value | text | can be empty |
| created_at | timestamp | |

The service writes an event in the same transaction as the change. The detail
window shows the events and the comments in one line of time.

Reason: "who closed this, and when" is the first question about a ticket.

### setting

| Column | Type | Note |
| ------ | ---- | ---- |
| key | text | primary key |
| value | text | |

The first key is `workspace_name`.

## Authentication

- `hotdogstand create-user --admin` makes the first account. The web page has
  no sign up form.
- An admin makes the other accounts in the "Users" window.
- The session cookie is `HttpOnly`, `SameSite=Lax`, and `Secure` when the
  server runs behind HTTPS.
- Each form that changes data carries a CSRF token.
- A failed log in waits a short time before it answers. A log in reports one
  message for a wrong name and for a wrong password.

## The list view

The ticket list is a 7.css list view in a window. A click on a column header
sorts by that column. A second click turns the order around. A toolbar above
the list filters by status, priority, assignee, and label, and has a search
field for the title.

The sort and the filter are in the address, for example
`/tickets?status=open&assignee=me&sort=-priority`. A person can save the
address and send it to another person. `tickets/query.py` reads these
parameters and refuses a column it does not know.

## The detail window

A double click on a row opens the ticket in a new window on the desktop.
Each open window has a button on the taskbar. More than one ticket can be open
at the same time. The window has tabs: "General" for the fields, and
"History" for the comments and the events.

## Styling

- 7.css draws the controls: the window frame, the buttons, the tabs, the list
  view, the fields, and the menus. The project keeps a copy of it in
  `web/static/css/` with its licence, so the server needs no CDN.
- `app.css` draws the rest: the desktop, the taskbar, the start menu, and the
  layout inside each window. It uses the class names of 7.css and adds few of
  its own.
- The colors and the sizes are CSS custom properties at the top of `app.css`.
  A theme changes the properties and nothing else.
- The first extra theme is "Hot Dog Stand", the red and yellow scheme of
  Windows 3.1, which gives the project its name.
- Segoe UI is the font when the computer has it. The project does not ship it,
  because its licence does not permit that. The fallback is the font of the
  system.
- The project draws its own icons, or takes them from a set with an open
  licence. It uses no icon, wallpaper, logo, or sound from Microsoft.
- Keyboard use works: Tab moves between the controls, Enter opens a ticket,
  and Escape closes the window that has the focus.

## Configuration

The server reads environment variables. Each one has a default for a computer
of one person.

| Variable | Default |
| -------- | ------- |
| `HOTDOGSTAND_DATABASE` | `./hotdogstand.db` |
| `HOTDOGSTAND_HOST` | `127.0.0.1` |
| `HOTDOGSTAND_PORT` | `8077` |
| `HOTDOGSTAND_SECURE_COOKIES` | `false` |

A variable that starts with `HOTDOGSTAND_` and that the server does not know
is an error. The message names the variable that it is probably.

## Tests

- A service test uses an SQLite database in memory, made inside the test.
- A route test uses the FastAPI test client, and checks the status code and
  the HTML that comes back.
- A test of the migrations runs every migration on an empty database, and
  then compares the result with the models.
- The CI runs `ruff format --check`, `ruff check`, and `pytest` on Linux,
  macOS, and Windows.

## Not in the first version

- More than one workspace or project.
- Log in with OAuth, LDAP, or SSO.
- E-mail notices.
- Attachments.
- A public JSON API.
- PostgreSQL.

[TODO.md](../TODO.md) holds these, with the reason for each.
