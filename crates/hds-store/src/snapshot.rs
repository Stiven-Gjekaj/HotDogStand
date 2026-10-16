//! A copy of the full workspace for the JSON export, and the import of that
//! copy into an empty workspace.

use hds_core::export::Snapshot;
use rusqlite::params;

use crate::{Error, Result, Store};

impl Store {
    /// Reads the full workspace in one transaction, so the copy agrees with
    /// itself.
    pub fn snapshot(&self) -> Result<Snapshot> {
        let now = self.now();
        // Each read below goes through the same connection, so each one is
        // inside this transaction.
        let tx = self.conn.unchecked_transaction()?;
        let mut snapshot = Snapshot::new(now);
        snapshot.settings = self.settings()?;
        snapshot.people = self.people()?;
        snapshot.labels = self.labels()?;
        snapshot.tickets = self.tickets()?;
        snapshot.comments = self.all_comments()?;
        snapshot.events = self.all_events()?;
        tx.commit()?;
        Ok(snapshot)
    }

    /// Writes an export into this workspace. Each record keeps its number, so
    /// a link such as `#12` in a comment stays correct. The workspace must be
    /// empty. When one record fails, nothing is written.
    pub fn import(&self, snapshot: &Snapshot) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let count: i64 = tx.query_row(
            "SELECT (SELECT count(*) FROM ticket) + (SELECT count(*) FROM person)
                  + (SELECT count(*) FROM label)",
            [],
            |row| row.get(0),
        )?;
        if count > 0 {
            return Err(Error::NotEmpty);
        }
        self.write_snapshot(snapshot).map_err(|error| match error {
            Error::Sqlite(error) => Error::BadExport(error.to_string()),
            error => error,
        })?;
        tx.commit()?;
        Ok(())
    }

    fn write_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let conn = &self.conn;
        for (key, value) in &snapshot.settings {
            self.set_setting(key, value)?;
        }
        for person in &snapshot.people {
            conn.execute(
                "INSERT INTO person (id, name, is_active, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    person.id,
                    person.name,
                    person.is_active,
                    person.created_at.to_string()
                ],
            )?;
        }
        for label in &snapshot.labels {
            conn.execute(
                "INSERT INTO label (id, name, color) VALUES (?1, ?2, ?3)",
                params![label.id, label.name, label.color],
            )?;
        }
        for ticket in &snapshot.tickets {
            conn.execute(
                "INSERT INTO ticket (id, title, description, status, priority, assignee_id,
                     created_at, updated_at, closed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    ticket.id,
                    ticket.title.as_str(),
                    ticket.description,
                    ticket.status.as_str(),
                    ticket.priority.as_str(),
                    ticket.assignee_id,
                    ticket.created_at.to_string(),
                    ticket.updated_at.to_string(),
                    ticket.closed_at.map(|t| t.to_string()),
                ],
            )?;
            for label_id in &ticket.label_ids {
                conn.execute(
                    "INSERT INTO ticket_label (ticket_id, label_id) VALUES (?1, ?2)",
                    params![ticket.id, label_id],
                )?;
            }
        }
        for comment in &snapshot.comments {
            conn.execute(
                "INSERT INTO comment (id, ticket_id, body, created_at, edited_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    comment.id,
                    comment.ticket_id,
                    comment.body,
                    comment.created_at.to_string(),
                    comment.edited_at.map(|t| t.to_string()),
                ],
            )?;
        }
        for event in &snapshot.events {
            conn.execute(
                "INSERT INTO event (id, ticket_id, kind, old_value, new_value, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    event.id,
                    event.ticket_id,
                    event.kind.as_str(),
                    event.old_value,
                    event.new_value,
                    event.created_at.to_string(),
                ],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hds_core::export::{read_json, write_json};
    use hds_core::{Edit, NewTicket, Status, Title};

    use crate::Error;
    use crate::test_support::store;

    /// A store with a person, a label, two tickets, comments, and a closed
    /// ticket, so that each table holds a row.
    fn full_store() -> crate::Store {
        let mut store = store();
        store.set_setting("workspace_name", "Office").unwrap();
        let ana = store.add_person("Ana").unwrap();
        let bug = store.add_label("bug", "#cc0000").unwrap();
        let a = store
            .create_ticket(NewTicket::new(Title::new("a").unwrap()))
            .unwrap();
        let b = store
            .create_ticket(NewTicket::new(Title::new("b").unwrap()))
            .unwrap();
        store
            .edit_ticket(a.id, Edit::Assignee(Some(ana.id)))
            .unwrap();
        store.edit_ticket(b.id, Edit::AddLabel(bug.id)).unwrap();
        store
            .edit_ticket(b.id, Edit::Status(Status::Closed))
            .unwrap();
        let comment = store.add_comment(a.id, "One").unwrap();
        store.edit_comment(comment.id, "One, edited").unwrap();
        store.add_comment(b.id, "See #1").unwrap();
        store
    }

    #[test]
    fn an_import_gives_the_same_workspace() {
        let source = full_store();
        let snapshot = source.snapshot().unwrap();

        let target = store();
        target.import(&snapshot).unwrap();
        let mut copy = target.snapshot().unwrap();
        copy.exported_at = snapshot.exported_at;
        assert_eq!(copy, snapshot);
    }

    #[test]
    fn a_new_ticket_after_an_import_gets_the_next_number() {
        let snapshot = full_store().snapshot().unwrap();
        let mut target = store();
        target.import(&snapshot).unwrap();
        let next = target
            .create_ticket(NewTicket::new(Title::new("c").unwrap()))
            .unwrap();
        assert_eq!(next.id, 3);
    }

    #[test]
    fn an_import_into_a_workspace_with_data_is_refused() {
        let snapshot = full_store().snapshot().unwrap();
        let target = store();
        target.add_label("mine", "#00ff00").unwrap();
        assert!(matches!(target.import(&snapshot), Err(Error::NotEmpty)));
        assert_eq!(target.labels().unwrap().len(), 1);
    }

    #[test]
    fn a_bad_export_writes_nothing() {
        let mut snapshot = full_store().snapshot().unwrap();
        // A comment on a ticket that the export does not hold.
        snapshot.comments[1].ticket_id = 99;
        let target = store();
        assert!(matches!(target.import(&snapshot), Err(Error::BadExport(_))));
        assert!(target.tickets().unwrap().is_empty());
        assert!(target.people().unwrap().is_empty());
        assert_eq!(target.setting("workspace_name").unwrap(), None);
    }

    #[test]
    fn the_snapshot_holds_everything_and_survives_json() {
        let mut store = store();
        store.set_setting("workspace_name", "Office").unwrap();
        let ana = store.add_person("Ana").unwrap();
        let bug = store.add_label("bug", "#cc0000").unwrap();
        let a = store
            .create_ticket(NewTicket::new(Title::new("a").unwrap()))
            .unwrap();
        let b = store
            .create_ticket(NewTicket::new(Title::new("b").unwrap()))
            .unwrap();
        store
            .edit_ticket(a.id, Edit::Assignee(Some(ana.id)))
            .unwrap();
        store.edit_ticket(b.id, Edit::AddLabel(bug.id)).unwrap();
        store
            .edit_ticket(b.id, Edit::Status(Status::Closed))
            .unwrap();
        store.add_comment(a.id, "One").unwrap();
        store.add_comment(b.id, "Two").unwrap();

        let snapshot = store.snapshot().unwrap();
        assert_eq!(snapshot.settings["workspace_name"], "Office");
        assert_eq!(snapshot.people, store.people().unwrap());
        assert_eq!(snapshot.labels, store.labels().unwrap());
        assert_eq!(snapshot.tickets, store.tickets().unwrap());
        assert_eq!(snapshot.comments.len(), 2);
        assert_eq!(snapshot.events.len(), 5);
        let mut all_events = store.events(a.id).unwrap();
        all_events.extend(store.events(b.id).unwrap());
        all_events.sort_by_key(|e| e.id);
        assert_eq!(snapshot.events, all_events);

        let mut file = Vec::new();
        write_json(&snapshot, &mut file).unwrap();
        assert_eq!(read_json(file.as_slice()).unwrap(), snapshot);
    }
}
