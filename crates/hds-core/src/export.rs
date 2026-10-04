//! The two export formats.
//!
//! JSON holds the full workspace, for a backup, a move to another computer,
//! or another tool. CSV holds the ticket list as the person sees it, for a
//! spreadsheet. Both write to any [`std::io::Write`], so this module opens no
//! file.

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{Comment, Event, Label, Person, Ticket};

/// The value of `format` at the top of a JSON export.
pub const FORMAT: &str = "hotdogstand";

/// The version of the shape of a JSON export. A change to the shape raises
/// it, so that an import can refuse a file that it does not understand.
pub const VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("The file is not an export of HotDogStand.")]
    NotAnExport,
    #[error(
        "The export has version {0}. This version of HotDogStand reads version {VERSION} or older."
    )]
    NewerVersion(u32),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Csv(#[from] csv::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// The full workspace at one time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub format: String,
    pub version: u32,
    pub exported_at: Timestamp,
    pub settings: BTreeMap<String, String>,
    pub people: Vec<Person>,
    pub labels: Vec<Label>,
    pub tickets: Vec<Ticket>,
    pub comments: Vec<Comment>,
    pub events: Vec<Event>,
}

impl Snapshot {
    /// Makes a snapshot with the current format and version.
    pub fn new(exported_at: Timestamp) -> Self {
        Snapshot {
            format: FORMAT.to_owned(),
            version: VERSION,
            exported_at,
            settings: BTreeMap::new(),
            people: Vec::new(),
            labels: Vec::new(),
            tickets: Vec::new(),
            comments: Vec::new(),
            events: Vec::new(),
        }
    }
}

pub fn write_json(snapshot: &Snapshot, out: impl Write) -> Result<(), ExportError> {
    serde_json::to_writer_pretty(out, snapshot)?;
    Ok(())
}

/// Reads a JSON export. It refuses a file that is not an export, and an
/// export from a newer version.
pub fn read_json(input: impl Read) -> Result<Snapshot, ExportError> {
    let value: serde_json::Value = serde_json::from_reader(input)?;
    if value.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
        return Err(ExportError::NotAnExport);
    }
    let version = value
        .get("version")
        .and_then(|v| v.as_u64())
        .ok_or(ExportError::NotAnExport)?;
    if version > u64::from(VERSION) {
        return Err(ExportError::NewerVersion(
            u32::try_from(version).unwrap_or(u32::MAX),
        ));
    }
    Ok(serde_json::from_value(value)?)
}

/// The header of the CSV export.
pub const CSV_HEADER: [&str; 9] = [
    "id", "title", "status", "priority", "assignee", "labels", "created", "updated", "closed",
];

/// Writes one row for each ticket, in the order given. The caller gives the
/// tickets in the order of the list view.
pub fn write_csv<'a>(
    tickets: impl IntoIterator<Item = &'a Ticket>,
    people: &[Person],
    labels: &[Label],
    out: impl Write,
) -> Result<(), ExportError> {
    let names: HashMap<i64, &str> = people.iter().map(|p| (p.id, p.name.as_str())).collect();
    let label_names: HashMap<i64, &str> = labels.iter().map(|l| (l.id, l.name.as_str())).collect();

    let mut writer = csv::Writer::from_writer(out);
    writer.write_record(CSV_HEADER)?;
    for ticket in tickets {
        let assignee = ticket
            .assignee_id
            .and_then(|id| names.get(&id).copied())
            .unwrap_or_default();
        let labels: Vec<&str> = ticket
            .label_ids
            .iter()
            .filter_map(|id| label_names.get(id).copied())
            .collect();
        writer.write_record([
            ticket.id.to_string().as_str(),
            ticket.title.as_str(),
            ticket.status.as_str(),
            ticket.priority.as_str(),
            assignee,
            labels.join("; ").as_str(),
            ticket.created_at.to_string().as_str(),
            ticket.updated_at.to_string().as_str(),
            ticket
                .closed_at
                .map(|t| t.to_string())
                .unwrap_or_default()
                .as_str(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventKind, NewEvent};
    use crate::{NewTicket, Priority, Status, Title};

    fn at(seconds: i64) -> Timestamp {
        Timestamp::from_second(seconds).unwrap()
    }

    fn snapshot() -> Snapshot {
        let mut ticket =
            NewTicket::new(Title::new("Printer, \"on fire\"").unwrap()).into_ticket(1, at(10));
        ticket.description = "Line one\nLine two".into();
        ticket.priority = Priority::Urgent;
        ticket.assignee_id = Some(3);
        ticket.label_ids = vec![5, 6];
        ticket.status = Status::Closed;
        ticket.closed_at = Some(at(20));

        let created = NewEvent::created();
        let mut snapshot = Snapshot::new(at(99));
        snapshot
            .settings
            .insert("workspace_name".into(), "Office".into());
        snapshot.people.push(Person {
            id: 3,
            name: "Ana".into(),
            is_active: false,
            created_at: at(1),
        });
        snapshot.labels.push(Label {
            id: 5,
            name: "bug".into(),
            color: "#cc0000".into(),
        });
        snapshot.labels.push(Label {
            id: 6,
            name: "hardware".into(),
            color: "#3a6ea5".into(),
        });
        snapshot.tickets.push(ticket);
        snapshot.comments.push(Comment {
            id: 1,
            ticket_id: 1,
            body: "It burns.".into(),
            created_at: at(11),
            edited_at: Some(at(12)),
        });
        snapshot.events.push(Event {
            id: 1,
            ticket_id: 1,
            kind: created.kind,
            old_value: None,
            new_value: None,
            created_at: at(10),
        });
        snapshot
    }

    #[test]
    fn json_comes_back_the_same() {
        let snapshot = snapshot();
        let mut file = Vec::new();
        write_json(&snapshot, &mut file).unwrap();
        assert_eq!(read_json(file.as_slice()).unwrap(), snapshot);
    }

    #[test]
    fn json_starts_with_the_format_and_the_version() {
        let mut file = Vec::new();
        write_json(&Snapshot::new(at(0)), &mut file).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&file).unwrap();
        assert_eq!(value["format"], "hotdogstand");
        assert_eq!(value["version"], 1);
    }

    #[test]
    fn json_uses_the_names_of_the_database() {
        let mut file = Vec::new();
        write_json(&snapshot(), &mut file).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&file).unwrap();
        let ticket = &value["tickets"][0];
        assert_eq!(ticket["status"], "closed");
        assert_eq!(ticket["priority"], "urgent");
        assert_eq!(ticket["created_at"], "1970-01-01T00:00:10Z");
        assert_eq!(value["events"][0]["kind"], EventKind::Created.as_str());
    }

    #[test]
    fn a_file_that_is_not_an_export_is_refused() {
        for text in [
            r#"{"tickets": []}"#,
            r#"{"format": "other", "version": 1}"#,
            r#"{"format": "hotdogstand"}"#,
        ] {
            assert!(
                matches!(read_json(text.as_bytes()), Err(ExportError::NotAnExport)),
                "{text}"
            );
        }
    }

    #[test]
    fn an_export_from_a_newer_version_is_refused() {
        let text = r#"{"format": "hotdogstand", "version": 2}"#;
        assert!(matches!(
            read_json(text.as_bytes()),
            Err(ExportError::NewerVersion(2))
        ));
    }

    #[test]
    fn csv_has_a_header_and_one_row_for_each_ticket() {
        let snapshot = snapshot();
        let mut file = Vec::new();
        write_csv(
            &snapshot.tickets,
            &snapshot.people,
            &snapshot.labels,
            &mut file,
        )
        .unwrap();

        let mut reader = csv::Reader::from_reader(file.as_slice());
        assert_eq!(reader.headers().unwrap(), CSV_HEADER.as_slice());
        let rows: Vec<csv::StringRecord> = reader.records().map(Result::unwrap).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0],
            csv::StringRecord::from(vec![
                "1",
                "Printer, \"on fire\"",
                "closed",
                "urgent",
                "Ana",
                "bug; hardware",
                "1970-01-01T00:00:10Z",
                "1970-01-01T00:00:10Z",
                "1970-01-01T00:00:20Z",
            ])
        );
    }

    #[test]
    fn csv_keeps_the_order_it_is_given() {
        let mut snapshot = snapshot();
        let mut second = snapshot.tickets[0].clone();
        second.id = 2;
        snapshot.tickets.push(second);

        let mut file = Vec::new();
        let order = [&snapshot.tickets[1], &snapshot.tickets[0]];
        write_csv(order, &[], &[], &mut file).unwrap();
        let ids: Vec<String> = csv::Reader::from_reader(file.as_slice())
            .records()
            .map(|r| r.unwrap()[0].to_owned())
            .collect();
        assert_eq!(ids, ["2", "1"]);
    }
}
