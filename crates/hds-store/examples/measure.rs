//! Measures the store and the query with a large workspace.
//!
//!     cargo run --release -p hds-store --example measure -- <path> [tickets]
//!
//! It makes a new workspace file at <path> with the given number of tickets
//! (10000 when no number is given), then times the reads that the
//! application does when it opens and when the person sorts or filters.
//! Point HOTDOGSTAND_WORKSPACE at the file to open it in the application.

use std::time::Instant;

use hds_core::query::{AssigneeFilter, Column, Sort, StatusFilter};
use hds_core::{Edit, NewTicket, Priority, Query, Status, Title};
use hds_store::Store;

const WORDS: [&str; 12] = [
    "printer", "server", "login", "paper", "chair", "network", "backup", "coffee", "wiki",
    "laptop", "badge", "phone",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("give the path of a new workspace file")?;
    let count: usize = args.next().map_or(Ok(10_000), |n| n.parse())?;
    let _ = std::fs::remove_file(&path);

    let started = Instant::now();
    let mut store = Store::open(path.as_ref())?;
    let people: Vec<i64> = ["Ana", "Ben", "Chloe", "Dev", "Eli"]
        .iter()
        .map(|n| store.add_person(n).map(|p| p.id))
        .collect::<Result<_, _>>()?;
    let labels: Vec<i64> = [
        ("bug", "#c42b1c"),
        ("ui", "#7a3ab0"),
        ("hardware", "#3a6ea5"),
    ]
    .iter()
    .map(|(n, c)| store.add_label(n, c).map(|l| l.id))
    .collect::<Result<_, _>>()?;
    for i in 0..count {
        let title = format!("Ticket {i} about the {}", WORDS[i % WORDS.len()]);
        let mut new = NewTicket::new(Title::new(&title)?);
        new.priority = Priority::ALL[i % 4];
        new.assignee_id = (i % 3 != 0).then(|| people[i % people.len()]);
        let id = store.create_ticket(new)?.id;
        if i % 2 == 0 {
            store.edit_ticket(id, Edit::AddLabel(labels[i % labels.len()]))?;
        }
        if i % 5 == 0 {
            store.edit_ticket(id, Edit::Status(Status::Closed))?;
        }
    }
    drop(store);
    println!("made {count} tickets in {:.1?}", started.elapsed());

    let started = Instant::now();
    let store = Store::open(path.as_ref())?;
    let tickets = store.tickets()?;
    let people = store.people()?;
    println!(
        "opened the file and read {} tickets in {:.1?}",
        tickets.len(),
        started.elapsed()
    );

    let queries = [
        ("the default view", Query::default()),
        (
            "sort by priority",
            Query {
                sort: Sort::default().clicked(Column::Priority),
                ..Query::default()
            },
        ),
        (
            "sort by assignee",
            Query {
                sort: Sort::default().clicked(Column::Assignee),
                ..Query::default()
            },
        ),
        ("search for a word", {
            let mut q = Query::default();
            q.filter.status = StatusFilter::All;
            q.filter.text = "printer".into();
            q
        }),
        ("filter by person and label", {
            let mut q = Query::default();
            // Ben has open tickets with the "ui" label.
            q.filter.assignee = AssigneeFilter::Person(people[1].id);
            q.filter.label = Some(2);
            q
        }),
    ];
    for (name, query) in queries {
        let started = Instant::now();
        let mut shown = 0;
        const RUNS: u32 = 20;
        for _ in 0..RUNS {
            shown = query.run(&tickets, &people).len();
        }
        println!(
            "{name}: {shown} tickets in {:.2?} (the mean of {RUNS} runs)",
            started.elapsed() / RUNS
        );
    }
    Ok(())
}
