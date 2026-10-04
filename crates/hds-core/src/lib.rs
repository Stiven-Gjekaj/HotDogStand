//! The rules of HotDogStand.
//!
//! This crate knows nothing about SQLite or Slint. A test of a rule opens no
//! file and no window.

mod error;
pub mod people;
pub mod ticket;

pub use error::Error;
pub use people::{Comment, Label, Person};
pub use ticket::{NewTicket, Priority, Status, Ticket, Title};
