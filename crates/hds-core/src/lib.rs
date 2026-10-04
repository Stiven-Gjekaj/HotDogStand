//! The rules of HotDogStand.
//!
//! This crate knows nothing about SQLite or Slint. A test of a rule opens no
//! file and no window.

mod error;
pub mod event;
pub mod export;
pub mod people;
pub mod query;
pub mod ticket;

pub use error::Error;
pub use event::{Edit, Event, EventKind, NewEvent, apply};
pub use people::{Comment, Label, Person};
pub use query::{AssigneeFilter, Column, Filter, Query, Sort, StatusFilter};
pub use ticket::{NewTicket, Priority, Status, Ticket, Title};
