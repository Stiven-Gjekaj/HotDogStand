//! A copy of the full workspace for the JSON export.

use hds_core::export::Snapshot;

use crate::{Result, Store};

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
}

#[cfg(test)]
mod tests {
    use hds_core::export::{read_json, write_json};
    use hds_core::{Edit, NewTicket, Status, Title};

    use crate::test_support::store;

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
