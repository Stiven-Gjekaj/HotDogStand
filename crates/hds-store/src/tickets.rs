//! The tickets, their labels, their comments, and their history.

use std::collections::HashMap;
use std::str::FromStr;

use hds_core::people::clean_comment;
use hds_core::{Comment, Edit, Event, NewEvent, NewTicket, Ticket, Title, apply};
use jiff::Timestamp;
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::people::not_found;
use crate::{Error, Result, Store, parse_time};

const TICKET_COLUMNS: &str = "id, title, description, status, priority, assignee_id, \
     created_at, updated_at, closed_at";

/// Reads a value with a rule of its own, such as a status or a title.
fn parse<T, E>(row: &Row<'_>, index: usize) -> rusqlite::Result<T>
where
    T: FromStr<Err = E>,
    E: std::error::Error + Send + Sync + 'static,
{
    let text: String = row.get(index)?;
    text.parse().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}

fn parse_time_at(row: &Row<'_>, index: usize) -> rusqlite::Result<Timestamp> {
    parse_time(&row.get::<_, String>(index)?)
}

fn parse_time_opt(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<Timestamp>> {
    row.get::<_, Option<String>>(index)?
        .as_deref()
        .map(parse_time)
        .transpose()
}

/// Reads a ticket with no labels. The caller adds them.
fn ticket(row: &Row<'_>) -> rusqlite::Result<Ticket> {
    let title: String = row.get(1)?;
    let title = Title::new(&title).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(1, Type::Text, Box::new(error))
    })?;
    Ok(Ticket {
        id: row.get(0)?,
        title,
        description: row.get(2)?,
        status: parse(row, 3)?,
        priority: parse(row, 4)?,
        assignee_id: row.get(5)?,
        label_ids: Vec::new(),
        created_at: parse_time_at(row, 6)?,
        updated_at: parse_time_at(row, 7)?,
        closed_at: parse_time_opt(row, 8)?,
    })
}

fn comment(row: &Row<'_>) -> rusqlite::Result<Comment> {
    Ok(Comment {
        id: row.get(0)?,
        ticket_id: row.get(1)?,
        body: row.get(2)?,
        created_at: parse_time_at(row, 3)?,
        edited_at: parse_time_opt(row, 4)?,
    })
}

fn event(row: &Row<'_>) -> rusqlite::Result<Event> {
    let kind: String = row.get(2)?;
    let kind = kind.parse().map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            Type::Text,
            format!("\"{kind}\" is not a kind of event").into(),
        )
    })?;
    Ok(Event {
        id: row.get(0)?,
        ticket_id: row.get(1)?,
        kind,
        old_value: row.get(3)?,
        new_value: row.get(4)?,
        created_at: parse_time_at(row, 5)?,
    })
}

fn read_ticket(conn: &Connection, id: i64) -> Result<Ticket> {
    let mut found = conn
        .query_row(
            &format!("SELECT {TICKET_COLUMNS} FROM ticket WHERE id = ?1"),
            [id],
            ticket,
        )
        .map_err(|error| not_found(error, "ticket", id))?;
    let mut statement =
        conn.prepare("SELECT label_id FROM ticket_label WHERE ticket_id = ?1 ORDER BY label_id")?;
    found.label_ids = statement
        .query_map([id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(found)
}

fn exists(conn: &Connection, table: &str, id: i64) -> Result<bool> {
    Ok(conn
        .query_row(
            &format!("SELECT 1 FROM {table} WHERE id = ?1"),
            [id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn check_person(conn: &Connection, id: Option<i64>) -> Result<()> {
    match id {
        Some(id) if !exists(conn, "person", id)? => Err(Error::NotFound { what: "person", id }),
        _ => Ok(()),
    }
}

fn write_event(
    conn: &Connection,
    ticket_id: i64,
    event: NewEvent,
    now: Timestamp,
) -> Result<Event> {
    conn.execute(
        "INSERT INTO event (ticket_id, kind, old_value, new_value, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            ticket_id,
            event.kind.as_str(),
            event.old_value,
            event.new_value,
            now.to_string()
        ],
    )?;
    Ok(Event {
        id: conn.last_insert_rowid(),
        ticket_id,
        kind: event.kind,
        old_value: event.old_value,
        new_value: event.new_value,
        created_at: now,
    })
}

fn touch(conn: &Connection, ticket_id: i64, now: Timestamp) -> Result<()> {
    conn.execute(
        "UPDATE ticket SET updated_at = ?2 WHERE id = ?1",
        params![ticket_id, now.to_string()],
    )?;
    Ok(())
}

impl Store {
    /// Gives every ticket with its labels, in the order of the ids.
    pub fn tickets(&self) -> Result<Vec<Ticket>> {
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {TICKET_COLUMNS} FROM ticket ORDER BY id"))?;
        let mut tickets: Vec<Ticket> = statement
            .query_map([], ticket)?
            .collect::<rusqlite::Result<_>>()?;

        let at: HashMap<i64, usize> = tickets.iter().enumerate().map(|(i, t)| (t.id, i)).collect();
        let mut statement = self
            .conn
            .prepare("SELECT ticket_id, label_id FROM ticket_label ORDER BY ticket_id, label_id")?;
        let pairs = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get(1)?)))?;
        for pair in pairs {
            let (ticket_id, label_id) = pair?;
            if let Some(&i) = at.get(&ticket_id) {
                tickets[i].label_ids.push(label_id);
            }
        }
        Ok(tickets)
    }

    pub fn ticket(&self, id: i64) -> Result<Ticket> {
        read_ticket(&self.conn, id)
    }

    /// Makes a ticket and its "created" event.
    pub fn create_ticket(&mut self, new: NewTicket) -> Result<Ticket> {
        let now = self.now();
        let tx = self.conn.transaction()?;
        check_person(&tx, new.assignee_id)?;
        tx.execute(
            "INSERT INTO ticket (title, description, status, priority, assignee_id,
                                 created_at, updated_at)
             VALUES (?1, ?2, 'open', ?3, ?4, ?5, ?5)",
            params![
                new.title.as_str(),
                new.description,
                new.priority.as_str(),
                new.assignee_id,
                now.to_string()
            ],
        )?;
        let ticket = new.into_ticket(tx.last_insert_rowid(), now);
        write_event(&tx, ticket.id, NewEvent::created(), now)?;
        tx.commit()?;
        Ok(ticket)
    }

    /// Applies one edit to a ticket and writes its event, in one transaction.
    /// An edit that changes nothing writes nothing and gives `None`.
    pub fn edit_ticket(&mut self, id: i64, edit: Edit) -> Result<Option<Event>> {
        let now = self.now();
        let tx = self.conn.transaction()?;
        let mut ticket = read_ticket(&tx, id)?;

        match edit {
            Edit::Assignee(assignee) => check_person(&tx, assignee)?,
            Edit::AddLabel(label) if !exists(&tx, "label", label)? => {
                return Err(Error::NotFound {
                    what: "label",
                    id: label,
                });
            }
            _ => {}
        }
        let label_change = match edit {
            Edit::AddLabel(label) => Some((label, true)),
            Edit::RemoveLabel(label) => Some((label, false)),
            _ => None,
        };

        let Some(new_event) = apply(&mut ticket, edit, now) else {
            return Ok(None);
        };

        tx.execute(
            "UPDATE ticket SET title = ?2, description = ?3, status = ?4, priority = ?5,
                               assignee_id = ?6, updated_at = ?7, closed_at = ?8
             WHERE id = ?1",
            params![
                id,
                ticket.title.as_str(),
                ticket.description,
                ticket.status.as_str(),
                ticket.priority.as_str(),
                ticket.assignee_id,
                ticket.updated_at.to_string(),
                ticket.closed_at.map(|t| t.to_string()),
            ],
        )?;
        match label_change {
            Some((label, true)) => {
                tx.execute(
                    "INSERT INTO ticket_label (ticket_id, label_id) VALUES (?1, ?2)",
                    params![id, label],
                )?;
            }
            Some((label, false)) => {
                tx.execute(
                    "DELETE FROM ticket_label WHERE ticket_id = ?1 AND label_id = ?2",
                    params![id, label],
                )?;
            }
            None => {}
        }
        let event = write_event(&tx, id, new_event, now)?;
        tx.commit()?;
        Ok(Some(event))
    }

    /// Gives the history of a ticket, oldest first.
    pub fn events(&self, ticket_id: i64) -> Result<Vec<Event>> {
        let mut statement = self.conn.prepare(
            "SELECT id, ticket_id, kind, old_value, new_value, created_at
             FROM event WHERE ticket_id = ?1 ORDER BY id",
        )?;
        let events = statement.query_map([ticket_id], event)?;
        Ok(events.collect::<rusqlite::Result<_>>()?)
    }

    /// Gives the comments of a ticket, oldest first.
    pub fn comments(&self, ticket_id: i64) -> Result<Vec<Comment>> {
        let mut statement = self.conn.prepare(
            "SELECT id, ticket_id, body, created_at, edited_at
             FROM comment WHERE ticket_id = ?1 ORDER BY id",
        )?;
        let comments = statement.query_map([ticket_id], comment)?;
        Ok(comments.collect::<rusqlite::Result<_>>()?)
    }

    fn comment(&self, id: i64) -> Result<Comment> {
        self.conn
            .query_row(
                "SELECT id, ticket_id, body, created_at, edited_at FROM comment WHERE id = ?1",
                [id],
                comment,
            )
            .map_err(|error| not_found(error, "comment", id))
    }

    /// Adds a comment. The ticket gets the time of the comment as the time of
    /// its last change.
    pub fn add_comment(&mut self, ticket_id: i64, body: &str) -> Result<Comment> {
        let body = clean_comment(body)?;
        let now = self.now();
        let tx = self.conn.transaction()?;
        if !exists(&tx, "ticket", ticket_id)? {
            return Err(Error::NotFound {
                what: "ticket",
                id: ticket_id,
            });
        }
        tx.execute(
            "INSERT INTO comment (ticket_id, body, created_at) VALUES (?1, ?2, ?3)",
            params![ticket_id, body, now.to_string()],
        )?;
        let id = tx.last_insert_rowid();
        touch(&tx, ticket_id, now)?;
        tx.commit()?;
        self.comment(id)
    }

    pub fn edit_comment(&mut self, id: i64, body: &str) -> Result<Comment> {
        let body = clean_comment(body)?;
        let old = self.comment(id)?;
        if old.body == body {
            return Ok(old);
        }
        let now = self.now();
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE comment SET body = ?2, edited_at = ?3 WHERE id = ?1",
            params![id, body, now.to_string()],
        )?;
        touch(&tx, old.ticket_id, now)?;
        tx.commit()?;
        self.comment(id)
    }
}

#[cfg(test)]
mod tests {
    use hds_core::{EventKind, Priority, Status};

    use super::*;
    use crate::test_support::store;

    fn new(title: &str) -> NewTicket {
        NewTicket::new(Title::new(title).unwrap())
    }

    #[test]
    fn a_new_ticket_comes_back_the_same_with_one_event() {
        let mut store = store();
        let made = store.create_ticket(new("Printer is on fire")).unwrap();
        assert_eq!(store.ticket(made.id).unwrap(), made);

        let events = store.events(made.id).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Created);
        assert_eq!(events[0].created_at, made.created_at);
    }

    #[test]
    fn ticket_numbers_start_at_one_and_go_up() {
        let mut store = store();
        let ids: Vec<_> = ["a", "b", "c"]
            .into_iter()
            .map(|t| store.create_ticket(new(t)).unwrap().id)
            .collect();
        assert_eq!(ids, [1, 2, 3]);
    }

    #[test]
    fn a_ticket_for_a_person_that_is_not_there_is_refused() {
        let mut store = store();
        let mut ticket = new("a");
        ticket.assignee_id = Some(5);
        assert!(matches!(
            store.create_ticket(ticket),
            Err(Error::NotFound {
                what: "person",
                id: 5
            })
        ));
        assert!(store.tickets().unwrap().is_empty());
    }

    #[test]
    fn each_edit_is_saved_with_its_event() {
        let mut store = store();
        let ana = store.add_person("Ana").unwrap();
        let bug = store.add_label("bug", "#cc0000").unwrap();
        let id = store.create_ticket(new("a")).unwrap().id;

        let edits = [
            Edit::Title(Title::new("b").unwrap()),
            Edit::Description("More words".into()),
            Edit::Priority(Priority::Urgent),
            Edit::Assignee(Some(ana.id)),
            Edit::AddLabel(bug.id),
            Edit::Status(Status::Closed),
        ];
        for edit in edits {
            assert!(store.edit_ticket(id, edit).unwrap().is_some());
        }

        let ticket = store.ticket(id).unwrap();
        assert_eq!(ticket.title.as_str(), "b");
        assert_eq!(ticket.description, "More words");
        assert_eq!(ticket.priority, Priority::Urgent);
        assert_eq!(ticket.assignee_id, Some(ana.id));
        assert_eq!(ticket.label_ids, vec![bug.id]);
        assert_eq!(ticket.status, Status::Closed);
        assert_eq!(ticket.closed_at, Some(ticket.updated_at));

        let kinds: Vec<_> = store
            .events(id)
            .unwrap()
            .into_iter()
            .map(|e| e.kind)
            .collect();
        assert_eq!(
            kinds,
            [
                EventKind::Created,
                EventKind::Title,
                EventKind::Description,
                EventKind::Priority,
                EventKind::Assigned,
                EventKind::Labeled,
                EventKind::Status,
            ]
        );
    }

    #[test]
    fn an_edit_that_changes_nothing_writes_nothing() {
        let mut store = store();
        let made = store.create_ticket(new("a")).unwrap();
        assert_eq!(
            store
                .edit_ticket(made.id, Edit::Status(Status::Open))
                .unwrap(),
            None
        );
        assert_eq!(store.ticket(made.id).unwrap(), made);
        assert_eq!(store.events(made.id).unwrap().len(), 1);
    }

    #[test]
    fn reopen_clears_the_closed_time_in_the_file() {
        let mut store = store();
        let id = store.create_ticket(new("a")).unwrap().id;
        store.edit_ticket(id, Edit::Status(Status::Closed)).unwrap();
        store.edit_ticket(id, Edit::Status(Status::Open)).unwrap();
        assert_eq!(store.ticket(id).unwrap().closed_at, None);
    }

    #[test]
    fn remove_a_label() {
        let mut store = store();
        let bug = store.add_label("bug", "#cc0000").unwrap();
        let ui = store.add_label("ui", "#0000cc").unwrap();
        let id = store.create_ticket(new("a")).unwrap().id;
        store.edit_ticket(id, Edit::AddLabel(ui.id)).unwrap();
        store.edit_ticket(id, Edit::AddLabel(bug.id)).unwrap();
        store.edit_ticket(id, Edit::RemoveLabel(ui.id)).unwrap();
        assert_eq!(store.ticket(id).unwrap().label_ids, vec![bug.id]);
    }

    #[test]
    fn an_edit_that_names_nothing_is_refused_and_writes_nothing() {
        let mut store = store();
        let made = store.create_ticket(new("a")).unwrap();
        assert!(matches!(
            store.edit_ticket(made.id, Edit::AddLabel(8)),
            Err(Error::NotFound {
                what: "label",
                id: 8
            })
        ));
        assert!(matches!(
            store.edit_ticket(made.id, Edit::Assignee(Some(8))),
            Err(Error::NotFound {
                what: "person",
                id: 8
            })
        ));
        assert!(matches!(
            store.edit_ticket(99, Edit::Priority(Priority::Low)),
            Err(Error::NotFound {
                what: "ticket",
                id: 99
            })
        ));
        assert_eq!(store.ticket(made.id).unwrap(), made);
        assert_eq!(store.events(made.id).unwrap().len(), 1);
    }

    #[test]
    fn the_list_holds_the_labels_of_each_ticket() {
        let mut store = store();
        let bug = store.add_label("bug", "#cc0000").unwrap();
        let ui = store.add_label("ui", "#0000cc").unwrap();
        let a = store.create_ticket(new("a")).unwrap().id;
        let b = store.create_ticket(new("b")).unwrap().id;
        store.edit_ticket(a, Edit::AddLabel(ui.id)).unwrap();
        store.edit_ticket(b, Edit::AddLabel(bug.id)).unwrap();
        store.edit_ticket(b, Edit::AddLabel(ui.id)).unwrap();

        let labels: Vec<_> = store
            .tickets()
            .unwrap()
            .into_iter()
            .map(|t| t.label_ids)
            .collect();
        assert_eq!(labels, [vec![ui.id], vec![bug.id, ui.id]]);
    }

    #[test]
    fn a_comment_moves_the_time_of_the_ticket() {
        let mut store = store();
        let made = store.create_ticket(new("a")).unwrap();
        let comment = store.add_comment(made.id, "\n It burns.\n").unwrap();
        assert_eq!(comment.body, "It burns.");
        assert_eq!(comment.edited_at, None);
        assert_eq!(
            store.ticket(made.id).unwrap().updated_at,
            comment.created_at
        );
        assert_eq!(store.comments(made.id).unwrap(), vec![comment]);
    }

    #[test]
    fn an_edited_comment_has_the_time_of_the_edit() {
        let mut store = store();
        let id = store.create_ticket(new("a")).unwrap().id;
        let comment = store.add_comment(id, "One").unwrap();
        let edited = store.edit_comment(comment.id, "Two").unwrap();
        assert_eq!(edited.body, "Two");
        assert!(edited.edited_at.unwrap() > comment.created_at);

        let same = store.edit_comment(comment.id, " Two ").unwrap();
        assert_eq!(same, edited);
    }

    #[test]
    fn a_comment_on_no_ticket_is_refused() {
        let mut store = store();
        assert!(matches!(
            store.add_comment(4, "Hello"),
            Err(Error::NotFound {
                what: "ticket",
                id: 4
            })
        ));
        assert!(matches!(
            store.add_comment(4, " "),
            Err(Error::Rule(hds_core::Error::EmptyComment))
        ));
    }
}
