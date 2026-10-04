//! The people, the labels, and the comments: the small records that a ticket
//! points to.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::Error;

/// A name that a ticket can be assigned to. It is not an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub id: i64,
    pub name: String,
    /// A person who leaves is hidden from the list of assignees, not deleted.
    pub is_active: bool,
    pub created_at: Timestamp,
}

/// Removes the spaces at the two ends of a name, and refuses an empty name.
pub fn clean_name(name: &str) -> Result<String, Error> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::EmptyName);
    }
    Ok(name.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    pub id: i64,
    pub name: String,
    /// A hex color in lower case, for example `#3a6ea5`.
    pub color: String,
}

/// Accepts `#rgb` and `#rrggbb` in any case, and gives `#rrggbb` in lower
/// case.
pub fn clean_color(color: &str) -> Result<String, Error> {
    let bad = || Error::BadColor(color.to_owned());
    let digits = color.trim().strip_prefix('#').ok_or_else(bad)?;
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(bad());
    }
    let full: String = match digits.len() {
        3 => digits.chars().flat_map(|c| [c, c]).collect(),
        6 => digits.to_owned(),
        _ => return Err(bad()),
    };
    Ok(format!("#{}", full.to_ascii_lowercase()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: i64,
    pub ticket_id: i64,
    /// Markdown.
    pub body: String,
    pub created_at: Timestamp,
    pub edited_at: Option<Timestamp>,
}

/// Removes the blank lines and spaces at the two ends of a comment, and
/// refuses an empty comment. The text inside stays as the person wrote it.
pub fn clean_comment(body: &str) -> Result<String, Error> {
    let body = body.trim();
    if body.is_empty() {
        return Err(Error::EmptyComment);
    }
    Ok(body.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_loses_the_spaces_at_its_ends() {
        assert_eq!(clean_name("  Ana Smith ").unwrap(), "Ana Smith");
    }

    #[test]
    fn an_empty_name_is_refused() {
        assert_eq!(clean_name("   "), Err(Error::EmptyName));
    }

    #[test]
    fn a_color_goes_to_six_digits_in_lower_case() {
        assert_eq!(clean_color("#3A6EA5").unwrap(), "#3a6ea5");
        assert_eq!(clean_color("#F0a").unwrap(), "#ff00aa");
        assert_eq!(clean_color(" #ffe866 ").unwrap(), "#ffe866");
    }

    #[test]
    fn a_color_that_is_not_hex_is_refused() {
        for bad in ["3a6ea5", "#3a6ea", "#3a6eag", "#", "red", "#ffff"] {
            assert_eq!(clean_color(bad), Err(Error::BadColor(bad.into())), "{bad}");
        }
    }

    #[test]
    fn a_comment_keeps_its_inner_lines() {
        assert_eq!(clean_comment("\n\nOne\n\nTwo\n\n").unwrap(), "One\n\nTwo");
    }

    #[test]
    fn an_empty_comment_is_refused() {
        assert_eq!(clean_comment(" \n "), Err(Error::EmptyComment));
    }
}
