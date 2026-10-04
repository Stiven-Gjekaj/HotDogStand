//! The schema of the workspace file.
//!
//! Each step is in the repository for ever. Do not change a step that is in a
//! release. Add a new step at the end. A workspace file from an old release
//! must open in a new one.

use rusqlite_migration::{M, Migrations};

const STEP_1: &str = "
CREATE TABLE person (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE CHECK (name <> ''),
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL
) STRICT;

CREATE TABLE label (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE CHECK (name <> ''),
    color TEXT NOT NULL CHECK (color GLOB '#[0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]')
) STRICT;

-- AUTOINCREMENT, so that the number of a ticket is never given again.
CREATE TABLE ticket (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL CHECK (title <> ''),
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK (status IN ('open', 'in_progress', 'closed')),
    priority TEXT NOT NULL CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
    assignee_id INTEGER REFERENCES person (id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    closed_at TEXT,
    CHECK ((status = 'closed') = (closed_at IS NOT NULL))
) STRICT;

CREATE TABLE ticket_label (
    ticket_id INTEGER NOT NULL REFERENCES ticket (id) ON DELETE CASCADE,
    label_id INTEGER NOT NULL REFERENCES label (id) ON DELETE CASCADE,
    PRIMARY KEY (ticket_id, label_id)
) STRICT, WITHOUT ROWID;

CREATE TABLE comment (
    id INTEGER PRIMARY KEY,
    ticket_id INTEGER NOT NULL REFERENCES ticket (id) ON DELETE CASCADE,
    body TEXT NOT NULL CHECK (body <> ''),
    created_at TEXT NOT NULL,
    edited_at TEXT
) STRICT;

CREATE INDEX comment_ticket ON comment (ticket_id, id);

CREATE TABLE event (
    id INTEGER PRIMARY KEY,
    ticket_id INTEGER NOT NULL REFERENCES ticket (id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    created_at TEXT NOT NULL
) STRICT;

CREATE INDEX event_ticket ON event (ticket_id, id);

CREATE TABLE setting (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;
";

pub(crate) fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(STEP_1)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steps_are_valid() {
        migrations().validate().unwrap();
    }
}
