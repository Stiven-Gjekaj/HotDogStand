//! Makes a workspace with a small office in it, for the screenshots of the
//! readme.
//!
//!     cargo run -p hds-store --example demo -- <path>
//!
//! The tickets, the comments, and the history happen over the two weeks
//! before now, so the list shows different times.

use std::cell::Cell;
use std::rc::Rc;

use hds_core::{Edit, NewTicket, Priority, Status, Title};
use hds_store::Store;
use jiff::{Timestamp, ToSpan};

struct Ticket {
    title: &'static str,
    description: &'static str,
    priority: Priority,
    assignee: Option<usize>,
    labels: &'static [usize],
    status: Status,
    comments: &'static [&'static str],
    /// Hours before now when the ticket was made.
    age: i64,
}

const PEOPLE: [&str; 4] = ["Ana Kovac", "Ben Okafor", "Chloe Martin", "Dev Patel"];
const LABELS: [(&str, &str); 6] = [
    ("bug", "#c42b1c"),
    ("hardware", "#3a6ea5"),
    ("network", "#1f7a8c"),
    ("docs", "#2e8b57"),
    ("request", "#c46a00"),
    ("ui", "#7a3ab0"),
];

const TICKETS: [Ticket; 12] = [
    Ticket {
        title: "The printer on floor 2 is on fire",
        description: "It started **this morning**.\n\n- The smoke alarm is *quiet*.\n- The paper tray is `empty`.",
        priority: Priority::Urgent,
        assignee: Some(0),
        labels: &[0, 1],
        status: Status::InProgress,
        comments: &[
            "I put a **fire extinguisher** next to it.\n\n- Do not print the *quarterly report*.\n- Use the printer on floor 3.",
        ],
        age: 5,
    },
    Ticket {
        title: "VPN drops every 20 minutes",
        description: "Since the update on Monday the VPN drops and asks for the password again.",
        priority: Priority::High,
        assignee: Some(3),
        labels: &[2, 0],
        status: Status::Open,
        comments: &["It happens on Wi-Fi only. On the cable it stays up."],
        age: 30,
    },
    Ticket {
        title: "Order more paper for the second floor",
        description: "Ten boxes of A4.",
        priority: Priority::Normal,
        assignee: Some(1),
        labels: &[4],
        status: Status::Open,
        comments: &[],
        age: 50,
    },
    Ticket {
        title: "Onboarding page of the wiki is out of date",
        description: "The page still names the old server and the old badge office.",
        priority: Priority::Low,
        assignee: Some(2),
        labels: &[3],
        status: Status::Open,
        comments: &[],
        age: 75,
    },
    Ticket {
        title: "Login screen plays a sound at full volume",
        description: "Every morning at 9:00 the whole room hears it.",
        priority: Priority::Normal,
        assignee: None,
        labels: &[5, 0],
        status: Status::Open,
        comments: &[],
        age: 100,
    },
    Ticket {
        title: "Move the server rack to the cool room",
        description: "The room next to the kitchen is too warm in the afternoon.",
        priority: Priority::High,
        assignee: Some(3),
        labels: &[1, 2],
        status: Status::InProgress,
        comments: &["The cool room has space for two racks."],
        age: 140,
    },
    Ticket {
        title: "Coffee machine makes tea",
        description: "Both buttons give tea.",
        priority: Priority::Low,
        assignee: Some(1),
        labels: &[1],
        status: Status::Open,
        comments: &["Some people say that this is not a bug."],
        age: 170,
    },
    Ticket {
        title: "New laptop for Chloe",
        description: "The old one does not charge.",
        priority: Priority::Normal,
        assignee: Some(0),
        labels: &[4, 1],
        status: Status::Open,
        comments: &[],
        age: 200,
    },
    Ticket {
        title: "Backup tapes from March are missing",
        description: "The box on the shelf holds February and April only.",
        priority: Priority::Urgent,
        assignee: Some(2),
        labels: &[0],
        status: Status::Open,
        comments: &[],
        age: 230,
    },
    Ticket {
        title: "Guest Wi-Fi password on the board is wrong",
        description: "",
        priority: Priority::Normal,
        assignee: Some(3),
        labels: &[2, 3],
        status: Status::Open,
        comments: &[],
        age: 260,
    },
    Ticket {
        title: "Replace the broken chair in room 4",
        description: "",
        priority: Priority::Low,
        assignee: None,
        labels: &[4],
        status: Status::Closed,
        comments: &[],
        age: 300,
    },
    Ticket {
        title: "Projector in the big room shows everything green",
        description: "",
        priority: Priority::High,
        assignee: Some(1),
        labels: &[1, 5],
        status: Status::Open,
        comments: &[],
        age: 330,
    },
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("give the path of a new workspace file")?;
    let _ = std::fs::remove_file(&path);

    let now = Timestamp::now();
    let clock = Rc::new(Cell::new(now - 340.hours()));
    let time = clock.clone();
    let mut store = Store::open(path.as_ref())?.with_clock(move || time.get());

    store.set_setting("workspace_name", "Office")?;
    let people: Vec<i64> = PEOPLE
        .iter()
        .map(|n| store.add_person(n).map(|p| p.id))
        .collect::<Result<_, _>>()?;
    let labels: Vec<i64> = LABELS
        .iter()
        .map(|(n, c)| store.add_label(n, c).map(|l| l.id))
        .collect::<Result<_, _>>()?;

    // The oldest ticket first, so that the numbers go up with the time.
    for ticket in TICKETS.iter().rev() {
        clock.set(now - ticket.age.hours());
        let mut new = NewTicket::new(Title::new(ticket.title)?);
        new.description = ticket.description.to_owned();
        let id = store.create_ticket(new)?.id;

        let later = |minutes: i64| clock.set(clock.get() + minutes.minutes());
        later(3);
        for &label in ticket.labels {
            store.edit_ticket(id, Edit::AddLabel(labels[label]))?;
        }
        if ticket.priority != Priority::Normal {
            later(12);
            store.edit_ticket(id, Edit::Priority(ticket.priority))?;
        }
        if let Some(person) = ticket.assignee {
            later(40);
            store.edit_ticket(id, Edit::Assignee(Some(people[person])))?;
        }
        for comment in ticket.comments {
            later(55);
            store.add_comment(id, comment)?;
        }
        if ticket.status != Status::Open {
            later(90);
            store.edit_ticket(id, Edit::Status(ticket.status))?;
        }
    }
    println!("made {} tickets in {path}", TICKETS.len());
    Ok(())
}
