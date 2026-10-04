//! The history of a ticket, and the rule that changes a ticket.
//!
//! Each change to a ticket goes through [`apply`]. It changes the ticket and
//! gives the event that records the change. The store writes the two in one
//! transaction, so the history always agrees with the data.

use std::fmt;
use std::str::FromStr;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::{Priority, Status, Ticket, Title};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Created,
    Title,
    Description,
    Status,
    Priority,
    Assigned,
    Labeled,
    Unlabeled,
}

impl EventKind {
    pub const ALL: [EventKind; 8] = [
        EventKind::Created,
        EventKind::Title,
        EventKind::Description,
        EventKind::Status,
        EventKind::Priority,
        EventKind::Assigned,
        EventKind::Labeled,
        EventKind::Unlabeled,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::Created => "created",
            EventKind::Title => "title",
            EventKind::Description => "description",
            EventKind::Status => "status",
            EventKind::Priority => "priority",
            EventKind::Assigned => "assigned",
            EventKind::Labeled => "labeled",
            EventKind::Unlabeled => "unlabeled",
        }
    }
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The text of an event kind that this version does not know. A workspace
/// file from a newer version can hold one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownEventKind(pub String);

impl FromStr for EventKind {
    type Err = UnknownEventKind;

    fn from_str(s: &str) -> Result<Self, UnknownEventKind> {
        EventKind::ALL
            .into_iter()
            .find(|kind| kind.as_str() == s)
            .ok_or_else(|| UnknownEventKind(s.to_owned()))
    }
}

/// An event that the store wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: i64,
    pub ticket_id: i64,
    pub kind: EventKind,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub created_at: Timestamp,
}

/// An event before the store gives it an id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEvent {
    pub kind: EventKind,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

impl NewEvent {
    fn new(kind: EventKind, old_value: Option<String>, new_value: Option<String>) -> Self {
        NewEvent {
            kind,
            old_value,
            new_value,
        }
    }

    pub fn created() -> Self {
        NewEvent::new(EventKind::Created, None, None)
    }
}

/// One change to a ticket.
///
/// An assignee and a label are ids. The rule does not check that the person or
/// the label exists. The store does that with its foreign keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    Title(Title),
    Description(String),
    Status(Status),
    Priority(Priority),
    Assignee(Option<i64>),
    AddLabel(i64),
    RemoveLabel(i64),
}

/// Applies an edit to a ticket and gives the event that records it.
///
/// An edit that changes nothing gives no event, and leaves `updated_at` as it
/// was.
pub fn apply(ticket: &mut Ticket, edit: Edit, now: Timestamp) -> Option<NewEvent> {
    let event = match edit {
        Edit::Title(title) => {
            if title == ticket.title {
                return None;
            }
            let old = std::mem::replace(&mut ticket.title, title);
            NewEvent::new(
                EventKind::Title,
                Some(old.into()),
                Some(ticket.title.to_string()),
            )
        }
        Edit::Description(text) => {
            if text == ticket.description {
                return None;
            }
            let old = std::mem::replace(&mut ticket.description, text);
            NewEvent::new(
                EventKind::Description,
                Some(old),
                Some(ticket.description.clone()),
            )
        }
        Edit::Status(status) => {
            if status == ticket.status {
                return None;
            }
            let old = std::mem::replace(&mut ticket.status, status);
            ticket.closed_at = (status == Status::Closed).then_some(now);
            NewEvent::new(
                EventKind::Status,
                Some(old.as_str().into()),
                Some(status.as_str().into()),
            )
        }
        Edit::Priority(priority) => {
            if priority == ticket.priority {
                return None;
            }
            let old = std::mem::replace(&mut ticket.priority, priority);
            NewEvent::new(
                EventKind::Priority,
                Some(old.as_str().into()),
                Some(priority.as_str().into()),
            )
        }
        Edit::Assignee(assignee) => {
            if assignee == ticket.assignee_id {
                return None;
            }
            let old = std::mem::replace(&mut ticket.assignee_id, assignee);
            NewEvent::new(
                EventKind::Assigned,
                old.map(|id| id.to_string()),
                assignee.map(|id| id.to_string()),
            )
        }
        Edit::AddLabel(label) => {
            let Err(at) = ticket.label_ids.binary_search(&label) else {
                return None;
            };
            ticket.label_ids.insert(at, label);
            NewEvent::new(EventKind::Labeled, None, Some(label.to_string()))
        }
        Edit::RemoveLabel(label) => {
            let Ok(at) = ticket.label_ids.binary_search(&label) else {
                return None;
            };
            ticket.label_ids.remove(at);
            NewEvent::new(EventKind::Unlabeled, Some(label.to_string()), None)
        }
    };
    ticket.updated_at = now;
    Some(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NewTicket;

    fn at(seconds: i64) -> Timestamp {
        Timestamp::from_second(seconds).unwrap()
    }

    fn ticket() -> Ticket {
        NewTicket::new(Title::new("Printer is on fire").unwrap()).into_ticket(1, at(0))
    }

    #[test]
    fn a_kind_goes_to_text_and_back() {
        for kind in EventKind::ALL {
            assert_eq!(kind.as_str().parse::<EventKind>(), Ok(kind));
        }
        assert_eq!(
            "renamed".parse::<EventKind>(),
            Err(UnknownEventKind("renamed".into()))
        );
    }

    #[test]
    fn a_new_title_records_the_old_and_the_new() {
        let mut t = ticket();
        let event = apply(
            &mut t,
            Edit::Title(Title::new("Printer is fine").unwrap()),
            at(5),
        );
        assert_eq!(
            event,
            Some(NewEvent::new(
                EventKind::Title,
                Some("Printer is on fire".into()),
                Some("Printer is fine".into())
            ))
        );
        assert_eq!(t.title.as_str(), "Printer is fine");
        assert_eq!(t.updated_at, at(5));
    }

    #[test]
    fn an_edit_that_changes_nothing_gives_no_event() {
        let mut t = ticket();
        let edits = [
            Edit::Title(t.title.clone()),
            Edit::Description(String::new()),
            Edit::Status(Status::Open),
            Edit::Priority(Priority::Normal),
            Edit::Assignee(None),
            Edit::RemoveLabel(3),
        ];
        for edit in edits {
            assert_eq!(apply(&mut t, edit.clone(), at(9)), None, "{edit:?}");
        }
        assert_eq!(t, ticket());
    }

    #[test]
    fn close_sets_the_time_and_reopen_clears_it() {
        let mut t = ticket();
        apply(&mut t, Edit::Status(Status::Closed), at(10));
        assert_eq!(t.closed_at, Some(at(10)));

        let event = apply(&mut t, Edit::Status(Status::InProgress), at(20)).unwrap();
        assert_eq!(t.closed_at, None);
        assert_eq!(event.old_value.as_deref(), Some("closed"));
        assert_eq!(event.new_value.as_deref(), Some("in_progress"));
    }

    #[test]
    fn a_status_that_is_not_closed_does_not_set_the_time() {
        let mut t = ticket();
        apply(&mut t, Edit::Status(Status::InProgress), at(10));
        assert_eq!(t.closed_at, None);
    }

    #[test]
    fn assign_and_unassign_record_the_ids() {
        let mut t = ticket();
        let event = apply(&mut t, Edit::Assignee(Some(4)), at(1)).unwrap();
        assert_eq!((event.old_value, event.new_value), (None, Some("4".into())));

        let event = apply(&mut t, Edit::Assignee(None), at(2)).unwrap();
        assert_eq!((event.old_value, event.new_value), (Some("4".into()), None));
        assert_eq!(t.assignee_id, None);
    }

    #[test]
    fn labels_stay_in_order_and_once() {
        let mut t = ticket();
        for label in [5, 2, 9, 2] {
            apply(&mut t, Edit::AddLabel(label), at(1));
        }
        assert_eq!(t.label_ids, vec![2, 5, 9]);

        let event = apply(&mut t, Edit::RemoveLabel(5), at(2)).unwrap();
        assert_eq!(event.kind, EventKind::Unlabeled);
        assert_eq!(t.label_ids, vec![2, 9]);
    }

    #[test]
    fn adding_a_label_that_is_there_gives_no_event() {
        let mut t = ticket();
        apply(&mut t, Edit::AddLabel(2), at(1));
        assert_eq!(apply(&mut t, Edit::AddLabel(2), at(2)), None);
        assert_eq!(t.updated_at, at(1));
    }
}
