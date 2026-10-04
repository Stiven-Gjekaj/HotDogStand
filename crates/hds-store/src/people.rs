//! The people and the labels.

use hds_core::people::{clean_color, clean_name};
use hds_core::{Label, Person};
use rusqlite::{Row, params};

use crate::{Error, Result, Store, name_taken, parse_time};

const PERSON_COLUMNS: &str = "id, name, is_active, created_at";

fn person(row: &Row<'_>) -> rusqlite::Result<Person> {
    Ok(Person {
        id: row.get(0)?,
        name: row.get(1)?,
        is_active: row.get(2)?,
        created_at: parse_time(&row.get::<_, String>(3)?)?,
    })
}

fn label(row: &Row<'_>) -> rusqlite::Result<Label> {
    Ok(Label {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
    })
}

impl Store {
    /// Gives every person, the hidden ones also, in the order of their names.
    pub fn people(&self) -> Result<Vec<Person>> {
        let mut statement = self.conn.prepare(&format!(
            "SELECT {PERSON_COLUMNS} FROM person ORDER BY name COLLATE NOCASE, id"
        ))?;
        let people = statement.query_map([], person)?;
        Ok(people.collect::<rusqlite::Result<_>>()?)
    }

    pub fn person(&self, id: i64) -> Result<Person> {
        self.conn
            .query_row(
                &format!("SELECT {PERSON_COLUMNS} FROM person WHERE id = ?1"),
                [id],
                person,
            )
            .map_err(|error| not_found(error, "person", id))
    }

    pub fn add_person(&self, name: &str) -> Result<Person> {
        let name = clean_name(name)?;
        let now = self.now();
        self.conn
            .execute(
                "INSERT INTO person (name, created_at) VALUES (?1, ?2)",
                params![name, now.to_string()],
            )
            .map_err(name_taken("person", &name))?;
        self.person(self.conn.last_insert_rowid())
    }

    pub fn rename_person(&self, id: i64, name: &str) -> Result<Person> {
        let name = clean_name(name)?;
        let changed = self
            .conn
            .execute(
                "UPDATE person SET name = ?2 WHERE id = ?1",
                params![id, name],
            )
            .map_err(name_taken("person", &name))?;
        if changed == 0 {
            return Err(Error::NotFound { what: "person", id });
        }
        self.person(id)
    }

    /// Hides or shows a person in the list of assignees. The tickets of a
    /// hidden person keep the name.
    pub fn set_person_active(&self, id: i64, active: bool) -> Result<Person> {
        let changed = self.conn.execute(
            "UPDATE person SET is_active = ?2 WHERE id = ?1",
            params![id, active],
        )?;
        if changed == 0 {
            return Err(Error::NotFound { what: "person", id });
        }
        self.person(id)
    }

    /// Gives every label in the order of their names.
    pub fn labels(&self) -> Result<Vec<Label>> {
        let mut statement = self
            .conn
            .prepare("SELECT id, name, color FROM label ORDER BY name COLLATE NOCASE, id")?;
        let labels = statement.query_map([], label)?;
        Ok(labels.collect::<rusqlite::Result<_>>()?)
    }

    pub fn label(&self, id: i64) -> Result<Label> {
        self.conn
            .query_row(
                "SELECT id, name, color FROM label WHERE id = ?1",
                [id],
                label,
            )
            .map_err(|error| not_found(error, "label", id))
    }

    pub fn add_label(&self, name: &str, color: &str) -> Result<Label> {
        let name = clean_name(name)?;
        let color = clean_color(color)?;
        self.conn
            .execute(
                "INSERT INTO label (name, color) VALUES (?1, ?2)",
                params![name, color],
            )
            .map_err(name_taken("label", &name))?;
        self.label(self.conn.last_insert_rowid())
    }

    pub fn edit_label(&self, id: i64, name: &str, color: &str) -> Result<Label> {
        let name = clean_name(name)?;
        let color = clean_color(color)?;
        let changed = self
            .conn
            .execute(
                "UPDATE label SET name = ?2, color = ?3 WHERE id = ?1",
                params![id, name, color],
            )
            .map_err(name_taken("label", &name))?;
        if changed == 0 {
            return Err(Error::NotFound { what: "label", id });
        }
        self.label(id)
    }
}

/// Gives a [`Error::NotFound`] when a query for one row found none.
pub(crate) fn not_found(error: rusqlite::Error, what: &'static str, id: i64) -> Error {
    match error {
        rusqlite::Error::QueryReturnedNoRows => Error::NotFound { what, id },
        error => Error::Sqlite(error),
    }
}

#[cfg(test)]
mod tests {
    use hds_core::Error as RuleError;

    use crate::Error;
    use crate::test_support::store;

    #[test]
    fn people_come_back_in_the_order_of_their_names() {
        let store = store();
        for name in ["zoe", "Adam", "mia"] {
            store.add_person(name).unwrap();
        }
        let names: Vec<_> = store
            .people()
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, ["Adam", "mia", "zoe"]);
    }

    #[test]
    fn a_new_person_is_active_and_has_the_time() {
        let store = store();
        let ana = store.add_person("  Ana ").unwrap();
        assert_eq!(ana.name, "Ana");
        assert!(ana.is_active);
        assert_eq!(ana.created_at.as_second(), 1000);
    }

    #[test]
    fn two_people_cannot_have_one_name_in_any_case() {
        let store = store();
        store.add_person("Ana").unwrap();
        let error = store.add_person("ANA").unwrap_err();
        assert!(
            matches!(error, Error::NameTaken { what: "person", ref name } if name == "ANA"),
            "{error:?}"
        );
    }

    #[test]
    fn rename_and_hide_a_person() {
        let store = store();
        let ana = store.add_person("Ana").unwrap();
        assert_eq!(store.rename_person(ana.id, "Anna").unwrap().name, "Anna");
        assert!(!store.set_person_active(ana.id, false).unwrap().is_active);
        assert!(store.set_person_active(ana.id, true).unwrap().is_active);
    }

    #[test]
    fn a_person_that_is_not_there_is_named_in_the_error() {
        let store = store();
        assert!(matches!(
            store.rename_person(9, "X"),
            Err(Error::NotFound {
                what: "person",
                id: 9
            })
        ));
        assert!(matches!(
            store.person(9),
            Err(Error::NotFound {
                what: "person",
                id: 9
            })
        ));
    }

    #[test]
    fn an_empty_name_is_refused_before_the_database() {
        let store = store();
        assert!(matches!(
            store.add_person(" "),
            Err(Error::Rule(RuleError::EmptyName))
        ));
    }

    #[test]
    fn a_label_keeps_a_clean_color() {
        let store = store();
        let bug = store.add_label("bug", "#C00").unwrap();
        assert_eq!(bug.color, "#cc0000");
        let bug = store.edit_label(bug.id, "Bug", "#3a6ea5").unwrap();
        assert_eq!((bug.name.as_str(), bug.color.as_str()), ("Bug", "#3a6ea5"));
    }

    #[test]
    fn a_label_with_a_bad_color_is_refused() {
        let store = store();
        assert!(matches!(
            store.add_label("bug", "red"),
            Err(Error::Rule(RuleError::BadColor(_)))
        ));
        assert!(store.labels().unwrap().is_empty());
    }

    #[test]
    fn two_labels_cannot_have_one_name() {
        let store = store();
        store.add_label("bug", "#cc0000").unwrap();
        let ui = store.add_label("ui", "#00cc00").unwrap();
        assert!(matches!(
            store.edit_label(ui.id, "Bug", "#00cc00"),
            Err(Error::NameTaken { what: "label", .. })
        ));
    }
}
