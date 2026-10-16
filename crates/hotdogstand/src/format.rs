//! The text that the windows show: times, and the lines of the history.

use std::collections::HashMap;

use hds_core::{Event, EventKind, Priority, Status, Ticket};
use jiff::Timestamp;
use jiff::tz::TimeZone;

/// A time in the time zone of the person, to the minute.
pub fn time(at: Timestamp, zone: &TimeZone) -> String {
    at.to_zoned(zone.clone())
        .strftime("%Y-%m-%d %H:%M")
        .to_string()
}

/// The names that the history needs: people and labels by id.
pub struct Names<'a> {
    pub people: &'a HashMap<i64, String>,
    pub labels: &'a HashMap<i64, String>,
}

impl Names<'_> {
    fn person(&self, value: Option<&str>) -> String {
        name_of(self.people, value, "a removed person")
    }

    fn label(&self, value: Option<&str>) -> String {
        name_of(self.labels, value, "a removed label")
    }
}

fn name_of(names: &HashMap<i64, String>, value: Option<&str>, missing: &str) -> String {
    value
        .and_then(|v| v.parse::<i64>().ok())
        .and_then(|id| names.get(&id).cloned())
        .unwrap_or_else(|| missing.to_owned())
}

fn status_label(value: Option<&str>) -> &'static str {
    value
        .and_then(|v| v.parse::<Status>().ok())
        .map_or("?", Status::label)
}

fn priority_label(value: Option<&str>) -> &'static str {
    value
        .and_then(|v| v.parse::<Priority>().ok())
        .map_or("?", Priority::label)
}

/// One line that says what an event changed.
pub fn event_text(event: &Event, names: &Names<'_>) -> String {
    let old = event.old_value.as_deref();
    let new = event.new_value.as_deref();
    match event.kind {
        EventKind::Created => "Created the ticket".to_owned(),
        EventKind::Title => format!(
            "Title: \"{}\" to \"{}\"",
            old.unwrap_or_default(),
            new.unwrap_or_default()
        ),
        EventKind::Description => "Changed the description".to_owned(),
        EventKind::Status => match new.and_then(|v| v.parse::<Status>().ok()) {
            Some(Status::Closed) => "Closed the ticket".to_owned(),
            _ if old == Some(Status::Closed.as_str()) => {
                format!("Reopened the ticket as {}", status_label(new))
            }
            _ => format!("Status: {} to {}", status_label(old), status_label(new)),
        },
        EventKind::Priority => format!(
            "Priority: {} to {}",
            priority_label(old),
            priority_label(new)
        ),
        EventKind::Assigned => match (old, new) {
            (None, Some(_)) => format!("Assigned to {}", names.person(new)),
            (Some(_), None) => format!("Removed the assignment to {}", names.person(old)),
            _ => format!(
                "Assigned to {} in place of {}",
                names.person(new),
                names.person(old)
            ),
        },
        EventKind::Labeled => format!("Added the label {}", names.label(new)),
        EventKind::Unlabeled => format!("Removed the label {}", names.label(old)),
    }
}

/// The line under the fields of a ticket that gives its times.
pub fn dates(ticket: &Ticket, zone: &TimeZone) -> String {
    let mut text = format!(
        "Created {}. Changed {}.",
        time(ticket.created_at, zone),
        time(ticket.updated_at, zone)
    );
    if let Some(closed) = ticket.closed_at {
        text.push_str(&format!(" Closed {}.", time(closed, zone)));
    }
    text
}

/// The text of the status bar of the main window.
pub fn count(shown: usize, total: usize) -> String {
    let tickets = |n: usize| if n == 1 { "ticket" } else { "tickets" };
    if shown == total {
        format!("{total} {}", tickets(total))
    } else {
        format!(
            "{shown} {} shown. {total} {} in the workspace.",
            tickets(shown),
            tickets(total)
        )
    }
}

/// Turns the Markdown of a description or a comment into styled text. A text
/// that the parser refuses shows as it is.
pub fn markdown(text: &str) -> slint::StyledText {
    slint::StyledText::from_markdown(text)
        .unwrap_or_else(|_| slint::StyledText::from_plain_text(text))
}

/// The longest directory, in characters, that a menu of recent workspaces
/// shows. A longer directory keeps its last parts.
const RECENT_DIR_MAX: usize = 40;

/// The name of a workspace in a menu of recent workspaces: the name of the
/// file, then its directory, with `~` for the home directory.
pub fn recent_label(path: &std::path::Path, home: Option<&std::path::Path>) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) else {
        return name;
    };
    let dir = match home.and_then(|home| dir.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display()),
        None => dir.display().to_string(),
    };
    format!("{name} ({})", shorten_dir(&dir))
}

/// Makes a directory short enough for a menu. It removes the first parts and
/// puts `...` in their place.
fn shorten_dir(dir: &str) -> String {
    if dir.chars().count() <= RECENT_DIR_MAX {
        return dir.to_owned();
    }
    let sep = std::path::MAIN_SEPARATOR;
    let mut tail = String::new();
    for part in dir.rsplit(sep).filter(|p| !p.is_empty()) {
        let next = format!("{sep}{part}{tail}");
        if next.chars().count() + 3 > RECENT_DIR_MAX {
            break;
        }
        tail = next;
    }
    if tail.is_empty() {
        // One part is too long alone, so the label keeps its end.
        let chars: Vec<char> = dir.chars().collect();
        tail = chars[chars.len() - (RECENT_DIR_MAX - 3)..].iter().collect();
    }
    format!("...{tail}")
}

/// The name of a workspace when the file gives none: the name of the file
/// without its extension.
pub fn workspace_name(path: &std::path::Path) -> String {
    path.file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// True for a link that a click can open: a web page or an e-mail address.
/// A link to a file or to another program stays closed, because the text of
/// a ticket can come from an export that another person wrote.
pub fn link_is_safe(link: &str) -> bool {
    let link = link.trim().to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| link.starts_with(scheme) && link.len() > scheme.len())
}

/// Reads `#rrggbb`. A color that is not in this form gives gray.
pub fn hex_color(text: &str) -> (u8, u8, u8) {
    let digits = text.strip_prefix('#').unwrap_or_default();
    let part = |at: usize| {
        digits
            .get(at..at + 2)
            .and_then(|p| u8::from_str_radix(p, 16).ok())
    };
    match (digits.len(), part(0), part(2), part(4)) {
        (6, Some(r), Some(g), Some(b)) => (r, g, b),
        _ => (0x80, 0x80, 0x80),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: EventKind, old: Option<&str>, new: Option<&str>) -> Event {
        Event {
            id: 1,
            ticket_id: 1,
            kind,
            old_value: old.map(str::to_owned),
            new_value: new.map(str::to_owned),
            created_at: Timestamp::UNIX_EPOCH,
        }
    }

    fn text(kind: EventKind, old: Option<&str>, new: Option<&str>) -> String {
        let people = HashMap::from([(1, "Ana".to_owned()), (2, "Ben".to_owned())]);
        let labels = HashMap::from([(7, "bug".to_owned())]);
        let names = Names {
            people: &people,
            labels: &labels,
        };
        event_text(&event(kind, old, new), &names)
    }

    #[test]
    fn each_kind_of_event_has_a_line() {
        use EventKind::*;
        assert_eq!(text(Created, None, None), "Created the ticket");
        assert_eq!(text(Title, Some("a"), Some("b")), "Title: \"a\" to \"b\"");
        assert_eq!(
            text(Description, Some("a"), Some("b")),
            "Changed the description"
        );
        assert_eq!(
            text(Status, Some("open"), Some("in_progress")),
            "Status: Open to In progress"
        );
        assert_eq!(
            text(Status, Some("open"), Some("closed")),
            "Closed the ticket"
        );
        assert_eq!(
            text(Status, Some("closed"), Some("open")),
            "Reopened the ticket as Open"
        );
        assert_eq!(
            text(Priority, Some("normal"), Some("urgent")),
            "Priority: Normal to Urgent"
        );
        assert_eq!(text(Labeled, None, Some("7")), "Added the label bug");
        assert_eq!(text(Unlabeled, Some("7"), None), "Removed the label bug");
    }

    #[test]
    fn assignment_lines_use_names() {
        use EventKind::Assigned;
        assert_eq!(text(Assigned, None, Some("1")), "Assigned to Ana");
        assert_eq!(
            text(Assigned, Some("1"), None),
            "Removed the assignment to Ana"
        );
        assert_eq!(
            text(Assigned, Some("1"), Some("2")),
            "Assigned to Ben in place of Ana"
        );
        assert_eq!(
            text(Assigned, None, Some("9")),
            "Assigned to a removed person"
        );
    }

    #[test]
    fn a_time_is_shown_in_the_zone_to_the_minute() {
        let at = Timestamp::from_second(90_061).unwrap();
        assert_eq!(time(at, &TimeZone::UTC), "1970-01-02 01:01");
        let plus_two = TimeZone::fixed(jiff::tz::offset(2));
        assert_eq!(time(at, &plus_two), "1970-01-02 03:01");
    }

    #[test]
    fn the_count_says_when_a_filter_hides_tickets() {
        assert_eq!(count(1, 1), "1 ticket");
        assert_eq!(count(12, 12), "12 tickets");
        assert_eq!(count(1, 12), "1 ticket shown. 12 tickets in the workspace.");
    }

    #[test]
    fn markdown_that_does_not_parse_shows_as_plain_text() {
        let plain = slint::StyledText::from_plain_text("<font color=");
        assert_eq!(markdown("<font color="), plain);
        assert_ne!(
            markdown("**bold**"),
            slint::StyledText::from_plain_text("**bold**")
        );
    }

    #[test]
    fn a_recent_workspace_shows_its_name_and_its_directory() {
        use std::path::{MAIN_SEPARATOR, Path, PathBuf};
        let home = PathBuf::from(format!("{MAIN_SEPARATOR}home{MAIN_SEPARATOR}ana"));
        let docs = home.join("Documents").join("Office.hotdogstand");
        assert_eq!(
            recent_label(&docs, Some(&home)),
            format!("Office.hotdogstand (~{MAIN_SEPARATOR}Documents)")
        );
        assert_eq!(
            recent_label(&home.join("Home.db"), Some(&home)),
            "Home.db (~)"
        );
        let other = PathBuf::from(format!("{MAIN_SEPARATOR}srv{MAIN_SEPARATOR}Team.db"));
        assert_eq!(
            recent_label(&other, Some(&home)),
            format!("Team.db ({MAIN_SEPARATOR}srv)")
        );
        assert_eq!(recent_label(Path::new("Bare.db"), None), "Bare.db");
    }

    #[test]
    fn a_long_directory_keeps_its_last_parts() {
        use std::path::{MAIN_SEPARATOR as S, PathBuf};
        let dir =
            format!("{S}private{S}tmp{S}a-very-long-session-directory-name{S}scratchpad{S}ws");
        let path = PathBuf::from(&dir).join("Home.hotdogstand");
        let label = recent_label(&path, None);
        assert_eq!(label, format!("Home.hotdogstand (...{S}scratchpad{S}ws)"));

        let one_part = format!("{S}{}", "x".repeat(60));
        let label = recent_label(&PathBuf::from(&one_part).join("W.db"), None);
        let dir = label.trim_start_matches("W.db (").trim_end_matches(')');
        assert_eq!(dir.chars().count(), RECENT_DIR_MAX);
        assert!(dir.starts_with("...x"));
    }

    #[test]
    fn a_workspace_is_named_after_its_file() {
        assert_eq!(
            workspace_name(std::path::Path::new("/x/Office.hotdogstand")),
            "Office"
        );
        assert_eq!(
            workspace_name(std::path::Path::new("workspace.db")),
            "workspace"
        );
    }

    #[test]
    fn only_web_and_mail_links_open() {
        for link in [
            "https://example.com/manual",
            "HTTP://example.com",
            " mailto:ana@example.com",
        ] {
            assert!(link_is_safe(link), "{link}");
        }
        for link in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "ssh://server",
            "C:\\Windows\\system32\\calc.exe",
            "/Applications/Calculator.app",
            "https://",
            "",
        ] {
            assert!(!link_is_safe(link), "{link}");
        }
    }

    #[test]
    fn a_hex_color_is_read_or_is_gray() {
        assert_eq!(hex_color("#3a6ea5"), (0x3a, 0x6e, 0xa5));
        assert_eq!(hex_color("red"), (0x80, 0x80, 0x80));
        assert_eq!(hex_color("#12345"), (0x80, 0x80, 0x80));
    }
}
