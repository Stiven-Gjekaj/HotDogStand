//! Sort and filter the ticket list.
//!
//! The list view and the CSV export both use [`Query::run`], so the export
//! holds what the person sees.

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::{Person, Priority, Status, Ticket};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusFilter {
    All,
    /// Open and in progress. This is what a person wants to see first.
    #[default]
    NotClosed,
    Is(Status),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AssigneeFilter {
    #[default]
    Anyone,
    Nobody,
    Person(i64),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Filter {
    pub status: StatusFilter,
    pub priority: Option<Priority>,
    pub assignee: AssigneeFilter,
    pub label: Option<i64>,
    /// Words that the title must hold, in any case. A number, with or without
    /// `#`, also finds the ticket with that id.
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Column {
    #[default]
    Id,
    Title,
    Status,
    Priority,
    Assignee,
    Created,
    Updated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sort {
    pub column: Column,
    pub descending: bool,
}

impl Sort {
    /// The sort after a click on the header of `column`. A click on the
    /// column of the sort turns the order around. A click on a different
    /// column sorts by it, with the newest, the highest, or the first at the
    /// top.
    pub fn clicked(self, column: Column) -> Sort {
        if column == self.column {
            return Sort {
                column,
                descending: !self.descending,
            };
        }
        let descending = matches!(column, Column::Priority | Column::Created | Column::Updated);
        Sort { column, descending }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Query {
    pub filter: Filter,
    pub sort: Sort,
}

impl Query {
    /// Gives the positions in `tickets` of the tickets that pass the filter,
    /// in the order of the sort. Two tickets that are equal in the column of
    /// the sort go in the order of their ids.
    pub fn run(&self, tickets: &[Ticket], people: &[Person]) -> Vec<usize> {
        let names: HashMap<i64, &str> = people.iter().map(|p| (p.id, p.name.as_str())).collect();
        let needle = Needle::new(&self.filter.text);

        let mut found: Vec<usize> = tickets
            .iter()
            .enumerate()
            .filter(|(_, t)| self.filter.passes(t, &needle))
            .map(|(i, _)| i)
            .collect();

        found.sort_by(|&a, &b| {
            let (a, b) = (&tickets[a], &tickets[b]);
            let order = compare(self.sort.column, a, b, &names);
            let order = if self.sort.descending {
                order.reverse()
            } else {
                order
            };
            order.then(a.id.cmp(&b.id))
        });
        found
    }
}

impl Filter {
    fn passes(&self, ticket: &Ticket, needle: &Needle) -> bool {
        let status = match self.status {
            StatusFilter::All => true,
            StatusFilter::NotClosed => ticket.status != Status::Closed,
            StatusFilter::Is(status) => ticket.status == status,
        };
        let assignee = match self.assignee {
            AssigneeFilter::Anyone => true,
            AssigneeFilter::Nobody => ticket.assignee_id.is_none(),
            AssigneeFilter::Person(id) => ticket.assignee_id == Some(id),
        };
        status
            && assignee
            && self.priority.is_none_or(|p| ticket.priority == p)
            && self
                .label
                .is_none_or(|label| ticket.label_ids.contains(&label))
            && needle.finds(ticket)
    }
}

/// The search text, made ready once for the full list.
struct Needle {
    words: Vec<String>,
    id: Option<i64>,
}

impl Needle {
    fn new(text: &str) -> Self {
        let text = text.trim();
        let id = text.strip_prefix('#').unwrap_or(text).parse().ok();
        let words = text.split_whitespace().map(str::to_lowercase).collect();
        Needle { words, id }
    }

    fn finds(&self, ticket: &Ticket) -> bool {
        if self.id == Some(ticket.id) {
            return true;
        }
        let title = ticket.title.as_str().to_lowercase();
        self.words.iter().all(|word| title.contains(word.as_str()))
    }
}

fn compare(column: Column, a: &Ticket, b: &Ticket, names: &HashMap<i64, &str>) -> Ordering {
    match column {
        Column::Id => a.id.cmp(&b.id),
        Column::Title => compare_text(a.title.as_str(), b.title.as_str()),
        Column::Status => a.status.cmp(&b.status),
        Column::Priority => a.priority.cmp(&b.priority),
        Column::Assignee => {
            let name = |t: &Ticket| t.assignee_id.and_then(|id| names.get(&id).copied());
            // A ticket with no assignee goes after the tickets with one.
            match (name(a), name(b)) {
                (Some(x), Some(y)) => compare_text(x, y),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            }
        }
        Column::Created => a.created_at.cmp(&b.created_at),
        Column::Updated => a.updated_at.cmp(&b.updated_at),
    }
}

fn compare_text(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;

    use super::*;
    use crate::{NewTicket, Title};

    fn at(seconds: i64) -> Timestamp {
        Timestamp::from_second(seconds).unwrap()
    }

    fn ticket(id: i64, title: &str) -> Ticket {
        NewTicket::new(Title::new(title).unwrap()).into_ticket(id, at(id))
    }

    fn person(id: i64, name: &str) -> Person {
        Person {
            id,
            name: name.into(),
            is_active: true,
            created_at: at(0),
        }
    }

    fn ids(tickets: &[Ticket], found: Vec<usize>) -> Vec<i64> {
        found.into_iter().map(|i| tickets[i].id).collect()
    }

    fn run(query: &Query, tickets: &[Ticket], people: &[Person]) -> Vec<i64> {
        ids(tickets, query.run(tickets, people))
    }

    #[test]
    fn the_default_hides_closed_tickets_and_sorts_by_id() {
        let mut tickets = vec![ticket(3, "c"), ticket(1, "a"), ticket(2, "b")];
        tickets[1].status = Status::Closed;
        assert_eq!(run(&Query::default(), &tickets, &[]), vec![2, 3]);
    }

    #[test]
    fn status_all_and_status_is() {
        let mut tickets = vec![ticket(1, "a"), ticket(2, "b"), ticket(3, "c")];
        tickets[0].status = Status::Closed;
        tickets[1].status = Status::InProgress;

        let mut query = Query::default();
        query.filter.status = StatusFilter::All;
        assert_eq!(run(&query, &tickets, &[]), vec![1, 2, 3]);
        query.filter.status = StatusFilter::Is(Status::InProgress);
        assert_eq!(run(&query, &tickets, &[]), vec![2]);
    }

    #[test]
    fn filter_by_priority_assignee_and_label() {
        let mut tickets = vec![ticket(1, "a"), ticket(2, "b"), ticket(3, "c")];
        tickets[0].priority = Priority::Urgent;
        tickets[1].assignee_id = Some(7);
        tickets[2].label_ids = vec![4];

        let mut query = Query::default();
        query.filter.priority = Some(Priority::Urgent);
        assert_eq!(run(&query, &tickets, &[]), vec![1]);

        let mut query = Query::default();
        query.filter.assignee = AssigneeFilter::Person(7);
        assert_eq!(run(&query, &tickets, &[]), vec![2]);
        query.filter.assignee = AssigneeFilter::Nobody;
        assert_eq!(run(&query, &tickets, &[]), vec![1, 3]);

        let mut query = Query::default();
        query.filter.label = Some(4);
        assert_eq!(run(&query, &tickets, &[]), vec![3]);
    }

    #[test]
    fn search_finds_every_word_in_any_case() {
        let tickets = vec![
            ticket(1, "The printer is on fire"),
            ticket(2, "Order more paper for the printer"),
            ticket(3, "Fire drill on Friday"),
        ];
        let mut query = Query::default();
        query.filter.text = "PRINTER fire".into();
        assert_eq!(run(&query, &tickets, &[]), vec![1]);
        query.filter.text = "  ".into();
        assert_eq!(run(&query, &tickets, &[]), vec![1, 2, 3]);
    }

    #[test]
    fn search_finds_a_ticket_by_its_number() {
        let tickets = vec![ticket(1, "a"), ticket(42, "b")];
        let mut query = Query::default();
        for text in ["42", "#42", " #42 "] {
            query.filter.text = text.into();
            assert_eq!(run(&query, &tickets, &[]), vec![42], "{text:?}");
        }
    }

    #[test]
    fn sort_by_priority_puts_urgent_first_after_one_click() {
        let mut tickets = vec![ticket(1, "a"), ticket(2, "b"), ticket(3, "c")];
        tickets[0].priority = Priority::Low;
        tickets[2].priority = Priority::Urgent;

        let query = Query {
            sort: Sort::default().clicked(Column::Priority),
            ..Query::default()
        };
        assert_eq!(run(&query, &tickets, &[]), vec![3, 2, 1]);
    }

    #[test]
    fn a_second_click_turns_the_order_around() {
        let sort = Sort::default().clicked(Column::Title);
        assert_eq!(
            sort,
            Sort {
                column: Column::Title,
                descending: false
            }
        );
        assert!(sort.clicked(Column::Title).descending);
        // A click on a different column starts in its own order.
        assert!(!sort.clicked(Column::Title).clicked(Column::Id).descending);
        assert!(Sort::default().clicked(Column::Id).descending);
    }

    #[test]
    fn sort_by_title_ignores_case() {
        let tickets = vec![ticket(1, "banana"), ticket(2, "Apple"), ticket(3, "cherry")];
        let query = Query {
            sort: Sort {
                column: Column::Title,
                descending: false,
            },
            ..Query::default()
        };
        assert_eq!(run(&query, &tickets, &[]), vec![2, 1, 3]);
    }

    #[test]
    fn sort_by_assignee_uses_names_and_puts_nobody_last() {
        let mut tickets = vec![ticket(1, "a"), ticket(2, "b"), ticket(3, "c")];
        tickets[0].assignee_id = Some(10);
        tickets[2].assignee_id = Some(20);
        let people = [person(10, "Zoe"), person(20, "adam")];

        let mut query = Query {
            sort: Sort {
                column: Column::Assignee,
                descending: false,
            },
            ..Query::default()
        };
        assert_eq!(run(&query, &tickets, &people), vec![3, 1, 2]);
        query.sort.descending = true;
        assert_eq!(run(&query, &tickets, &people), vec![2, 1, 3]);
    }

    #[test]
    fn equal_values_keep_the_order_of_the_ids() {
        let tickets = vec![ticket(3, "a"), ticket(1, "a"), ticket(2, "a")];
        for descending in [false, true] {
            let query = Query {
                sort: Sort {
                    column: Column::Status,
                    descending,
                },
                ..Query::default()
            };
            assert_eq!(run(&query, &tickets, &[]), vec![1, 2, 3]);
        }
    }

    #[test]
    fn sort_by_updated_shows_the_newest_first() {
        let mut tickets = vec![ticket(1, "a"), ticket(2, "b")];
        tickets[0].updated_at = at(100);
        let query = Query {
            sort: Sort::default().clicked(Column::Updated),
            ..Query::default()
        };
        assert_eq!(run(&query, &tickets, &[]), vec![1, 2]);
    }
}
