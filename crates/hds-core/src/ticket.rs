use std::fmt;
use std::str::FromStr;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::Error;

/// The largest number of characters in a title.
pub const TITLE_MAX: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Open,
    InProgress,
    Closed,
}

impl Status {
    pub const ALL: [Status; 3] = [Status::Open, Status::InProgress, Status::Closed];

    /// The value in the database and in an export.
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::InProgress => "in_progress",
            Status::Closed => "closed",
        }
    }

    /// The text that a person sees.
    pub fn label(self) -> &'static str {
        match self {
            Status::Open => "Open",
            Status::InProgress => "In progress",
            Status::Closed => "Closed",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Status {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        Status::ALL
            .into_iter()
            .find(|status| status.as_str() == s)
            .ok_or_else(|| Error::UnknownStatus(s.to_owned()))
    }
}

/// The order of the variants is the order of a sort: low is first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

impl Priority {
    pub const ALL: [Priority; 4] = [
        Priority::Low,
        Priority::Normal,
        Priority::High,
        Priority::Urgent,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Priority::Low => "low",
            Priority::Normal => "normal",
            Priority::High => "high",
            Priority::Urgent => "urgent",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Priority::Low => "Low",
            Priority::Normal => "Normal",
            Priority::High => "High",
            Priority::Urgent => "Urgent",
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Priority {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        Priority::ALL
            .into_iter()
            .find(|priority| priority.as_str() == s)
            .ok_or_else(|| Error::UnknownPriority(s.to_owned()))
    }
}

/// A title that is not empty and not too long. The spaces at the two ends are
/// removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Title(String);

impl Title {
    pub fn new(text: &str) -> Result<Self, Error> {
        let text = text.trim();
        if text.is_empty() {
            return Err(Error::EmptyTitle);
        }
        let len = text.chars().count();
        if len > TITLE_MAX {
            return Err(Error::TitleTooLong {
                len,
                max: TITLE_MAX,
            });
        }
        Ok(Title(text.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Title {
    type Error = Error;

    fn try_from(text: String) -> Result<Self, Error> {
        Title::new(&text)
    }
}

impl From<Title> for String {
    fn from(title: Title) -> String {
        title.0
    }
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ticket {
    pub id: i64,
    pub title: Title,
    pub description: String,
    pub status: Status,
    pub priority: Priority,
    pub assignee_id: Option<i64>,
    /// The ids of the labels, in ascending order.
    pub label_ids: Vec<i64>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub closed_at: Option<Timestamp>,
}

/// The fields of a ticket that does not exist yet. The store gives the id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTicket {
    pub title: Title,
    pub description: String,
    pub priority: Priority,
    pub assignee_id: Option<i64>,
}

impl NewTicket {
    pub fn new(title: Title) -> Self {
        NewTicket {
            title,
            description: String::new(),
            priority: Priority::Normal,
            assignee_id: None,
        }
    }

    /// Makes the ticket with an id. A new ticket is always open.
    pub fn into_ticket(self, id: i64, now: Timestamp) -> Ticket {
        Ticket {
            id,
            title: self.title,
            description: self.description,
            status: Status::Open,
            priority: self.priority,
            assignee_id: self.assignee_id,
            label_ids: Vec::new(),
            created_at: now,
            updated_at: now,
            closed_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_status_goes_to_text_and_back() {
        for status in Status::ALL {
            assert_eq!(status.as_str().parse::<Status>(), Ok(status));
        }
    }

    #[test]
    fn an_unknown_status_names_the_text() {
        assert_eq!(
            "done".parse::<Status>(),
            Err(Error::UnknownStatus("done".into()))
        );
    }

    #[test]
    fn a_priority_goes_to_text_and_back() {
        for priority in Priority::ALL {
            assert_eq!(priority.as_str().parse::<Priority>(), Ok(priority));
        }
    }

    #[test]
    fn priorities_sort_from_low_to_urgent() {
        let mut list = vec![
            Priority::Urgent,
            Priority::Low,
            Priority::High,
            Priority::Normal,
        ];
        list.sort();
        assert_eq!(list, Priority::ALL.to_vec());
    }

    #[test]
    fn a_title_loses_the_spaces_at_its_ends() {
        assert_eq!(
            Title::new("  Fix the printer \n").unwrap().as_str(),
            "Fix the printer"
        );
    }

    #[test]
    fn an_empty_title_is_refused() {
        assert_eq!(Title::new(""), Err(Error::EmptyTitle));
        assert_eq!(Title::new(" \t "), Err(Error::EmptyTitle));
    }

    #[test]
    fn a_title_counts_characters_not_bytes() {
        let at_limit = "é".repeat(TITLE_MAX);
        assert!(Title::new(&at_limit).is_ok());
        let over = "é".repeat(TITLE_MAX + 1);
        assert_eq!(
            Title::new(&over),
            Err(Error::TitleTooLong {
                len: TITLE_MAX + 1,
                max: TITLE_MAX
            })
        );
    }

    #[test]
    fn a_new_ticket_is_open_with_no_labels() {
        let now = Timestamp::UNIX_EPOCH;
        let ticket = NewTicket::new(Title::new("A").unwrap()).into_ticket(7, now);
        assert_eq!(ticket.id, 7);
        assert_eq!(ticket.status, Status::Open);
        assert_eq!(ticket.priority, Priority::Normal);
        assert!(ticket.label_ids.is_empty());
        assert_eq!(ticket.created_at, now);
        assert_eq!(ticket.updated_at, now);
        assert_eq!(ticket.closed_at, None);
    }
}
