<div align="center">
  <a href="README.md"><img src="assets/hotdogstand.svg" alt="HotDogStand" height="44"></a>
</div>

# What is left to build

A task with a box is not built. A task without one is done.

## Where things are

Nothing is built. The plan is in [docs/architecture.md](docs/architecture.md).
The steps below are in the order to build them. Each step is one or more
commits, and each commit does one thing.

## The first version

### The skeleton

- [ ] `pyproject.toml`, the `hotdogstand` package, and an empty `cli.py`
- [ ] ruff and pytest, and a CI workflow that runs them on Linux, macOS, and
      Windows
- [ ] `config.py`, which reads the settings and refuses a setting it does not
      know

### The database

- [ ] The models: user, session, ticket, label, ticket_label, comment, event,
      setting
- [ ] The first Alembic migration, and `hotdogstand migrate`
- [ ] A test that runs every migration and compares the result with the models

### Accounts

- [ ] Hash and verify a password with Argon2id
- [ ] `hotdogstand create-user`, with `--admin`
- [ ] Log in, log out, and the session cookie
- [ ] The CSRF token on each form
- [ ] The "Users" window: an admin makes, deactivates, and activates a user

### Tickets

- [ ] Create a ticket, with an event
- [ ] Edit the title, the description, and the priority, with an event for
      each change
- [ ] Assign and unassign
- [ ] Close and reopen
- [ ] Add and remove a label, and the "Labels" window to make labels
- [ ] Add and edit a comment
- [ ] Markdown, with unsafe HTML removed

### The list view

- [ ] The list of tickets, with no sort and no filter
- [ ] Sort by each column, from the address
- [ ] Filter by status, priority, assignee, and label, from the address
- [ ] Search the title
- [ ] htmx replaces the list and not the page

### The desktop

- [ ] Vendor 7.css with its licence
- [ ] The desktop, the taskbar, and the start menu in `base.html` and
      `app.css`
- [ ] `windows.js`: open, move, focus, minimize, and close a window
- [ ] The detail window, with the tabs "General" and "History"
- [ ] The full page for each window when the browser runs no script
- [ ] Keyboard: Tab, Enter, and Escape
- [ ] The "Hot Dog Stand" theme
- [ ] The icons

### The release

- [ ] A screenshot in the readme
- [ ] Measure the time to start, and the time for the list with 10,000
      tickets, and write the numbers here
- [ ] Tag `v0.1.0`

## After the first version

Each of these waits for a person who needs it. A feature that nobody uses rots.

- [ ] More than one workspace. One team is the first user.
- [ ] Log in with OAuth or LDAP. A small team can make its accounts by hand.
- [ ] E-mail notices. This needs an SMTP setting and a queue, and the first
      version has neither.
- [ ] Attachments. This needs a directory for files, a limit on size, and a
      check on the type.
- [ ] A public JSON API. The routes return HTML first. The services are ready
      for an API when one is necessary.
- [ ] PostgreSQL. SQLAlchemy makes this possible. SQLite is enough for one
      team.
- [ ] A Docker image.
