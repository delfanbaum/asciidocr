use serde::Serialize;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::graph::inlines::Inline;

#[derive(Clone, Copy, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum NodeTypes {
    Block,
    Inline,
    String,
}

/// Struct containing document header information
#[derive(Serialize, Clone, Debug)]
pub struct Header {
    pub title: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<Vec<Author>>,
    pub location: Vec<Location>,
    /// the revision line (`v1.2, 2026-01-15: remark`); the ASG schema has no place for it, so it
    /// isn't serialized (the backends get it from here, and `{revnumber}` and the like work)
    #[serde(skip)]
    pub revision: Option<Revision>,
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

impl Header {
    pub fn new() -> Self {
        Header {
            title: vec![],
            authors: None,
            location: vec![],
            revision: None,
        }
    }

    /// The attributes the author and revision lines imply, as Asciidoctor names them: `author`,
    /// `email`, `firstname`, `middlename`, `lastname` and `authorinitials` for the first author
    /// (with a `_2`, `_3`, ... suffix for the others), `authors` and `authorcount`, and `revnumber`,
    /// `revdate` and `revremark`
    pub fn implied_attributes(&self) -> Vec<(String, String)> {
        let mut attributes = vec![];
        let authors = self.authors.clone().unwrap_or_default();
        for (idx, author) in authors.iter().enumerate() {
            let suffix = if idx == 0 {
                String::new()
            } else {
                format!("_{}", idx + 1)
            };
            for (name, value) in [
                ("author", &author.fullname),
                ("email", &author.address),
                ("firstname", &author.firstname),
                ("middlename", &author.middlename),
                ("lastname", &author.lastname),
                ("authorinitials", &author.initials),
            ] {
                if !value.is_empty() {
                    attributes.push((format!("{name}{suffix}"), value.clone()));
                }
            }
        }
        if !authors.is_empty() {
            let names: Vec<&str> = authors.iter().map(|a| a.fullname.as_str()).collect();
            attributes.push(("authors".into(), names.join(", ")));
            attributes.push(("authorcount".into(), authors.len().to_string()));
        }
        if let Some(revision) = &self.revision {
            for (name, value) in [
                ("revnumber", &revision.number),
                ("revdate", &revision.date),
                ("revremark", &revision.remark),
            ] {
                if !value.is_empty() {
                    attributes.push((name.into(), value.clone()));
                }
            }
        }
        attributes
    }

    pub fn is_empty(&self) -> bool {
        self.title.is_empty() && self.authors.is_none()
    }

    pub fn consolidate(&mut self) {
        if let Some(last_inline) = self.title.last_mut() {
            self.location = Location::reconcile(self.location.clone(), last_inline.locations())
        }
    }

    pub fn title(&self) -> Vec<Inline> {
        self.title.clone()
    }

    /// Returns a document_id from the title, otherwise an empty string
    pub fn document_id(&self) -> String {
        let mut id = String::new();
        for inline in self.title() {
            id.push_str(&inline.extract_values_to_string());
        }
        // replace non-alphanumeric characters
        id = id
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        id.to_lowercase().to_string()
    }
}

/// An author, from the author line under the document title, as the ASG schema has it (each
/// field optional)
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Author {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub fullname: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub initials: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub firstname: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub middlename: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub lastname: String,
    /// the email address
    #[serde(skip_serializing_if = "String::is_empty")]
    pub address: String,
}

impl Author {
    /// The authors on an author line (`Jane Q. Doe <jane@example.com>; John Smith`), read as
    /// Asciidoctor reads them: up to three names (first, middle, last; an underscore in one is a
    /// space) and an optional email address in angle brackets; anything else is one full name
    pub fn from_line(line: &str) -> Vec<Author> {
        line.split(';')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(Author::from_entry)
            .collect()
    }

    fn from_entry(entry: &str) -> Author {
        static AUTHOR: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^(\w[\w\-'.]*)(?: +(\w[\w\-'.]*))?(?: +(\w[\w\-'.]*))?(?: +<([^>]+)>)?$")
                .unwrap()
        });
        let Some(parts) = AUTHOR.captures(entry) else {
            let fullname = entry.split_whitespace().collect::<Vec<_>>().join(" ");
            return Author {
                initials: fullname
                    .chars()
                    .next()
                    .map(String::from)
                    .unwrap_or_default(),
                firstname: fullname.clone(),
                fullname,
                ..Author::default()
            };
        };
        let name = |idx: usize| {
            parts
                .get(idx)
                .map(|part| part.as_str().replace('_', " "))
                .unwrap_or_default()
        };
        let firstname = name(1);
        let (middlename, lastname) = match parts.get(3) {
            Some(_) => (name(2), name(3)),
            None => (String::new(), name(2)),
        };
        let names = [&firstname, &middlename, &lastname];
        Author {
            fullname: names
                .iter()
                .filter(|name| !name.is_empty())
                .map(|name| name.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            initials: names
                .iter()
                .filter_map(|name| name.chars().next())
                .collect(),
            firstname,
            middlename,
            lastname,
            address: parts
                .get(4)
                .map(|address| address.as_str().to_string())
                .unwrap_or_default(),
        }
    }
}

/// The revision line under the author line: `v1.2, 2026-01-15: A remark`, or any of its parts
/// (a lone `v1.0` is a number, a lone date a date)
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Revision {
    /// `revnumber`, without the `v` (or `version `) before it
    pub number: String,
    /// `revdate`
    pub date: String,
    /// `revremark`
    pub remark: String,
}

impl Revision {
    /// Reads a revision line as Asciidoctor does
    pub fn from_line(line: &str) -> Revision {
        let line = line.trim();
        // a remark follows a colon (not one at the start of the line)
        let (head, remark) = match line.char_indices().find(|(idx, c)| *c == ':' && *idx > 0) {
            Some((idx, _)) => (&line[..idx], line[idx + 1..].trim().to_string()),
            None => (line, String::new()),
        };
        let head = head.trim_end();
        let (number, component) = match head.split_once(',') {
            // before a comma: the number, after whatever (`v`, `version `) comes before its digits
            Some((number, rest)) => (
                number
                    .trim_start_matches(|c: char| !c.is_ascii_digit() && c != '{')
                    .trim_end()
                    .to_string(),
                rest.trim(),
            ),
            None => (String::new(), head.trim()),
        };
        let mut revision = Revision {
            number,
            remark,
            ..Revision::default()
        };
        if !component.is_empty() {
            match component.strip_prefix('v') {
                Some(number) if revision.number.is_empty() => revision.number = number.to_string(),
                _ => revision.date = component.to_string(),
            }
        }
        revision
    }
}

/// A "location" pertaining to a given document object, usually the start or end of something
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Location {
    pub line: usize, // 1-indexed
    pub col: usize,  // 1-indexed
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub file: Vec<String>,
}

impl Default for Location {
    fn default() -> Self {
        Location {
            line: 1,
            col: 1,
            file: vec![],
        }
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.file.len() == other.file.len() {
            if self.line == other.line {
                self.col.cmp(&other.col)
            } else {
                self.line.cmp(&other.line)
            }
        } else {
            self.file.len().cmp(&other.file.len())
        }
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Location {
    pub fn new(line: usize, col: usize, file: Vec<String>) -> Self {
        Location { line, col, file }
    }

    pub fn col(&self) -> usize {
        self.col
    }

    pub fn reconcile(mut start: Vec<Location>, other: Vec<Location>) -> Vec<Location> {
        if !other.is_empty() {
            start.extend(other);
            start.sort();
            if start.len() > 1 {
                // remove the middle
                start.drain(1..start.len() - 1);
            }
        }
        start
    }

    pub fn destructure_inline_locations(locations: Vec<Location>) -> (usize, usize, usize) {
        let mut line: usize = 0;
        let mut startcol: usize = 0;
        let mut endcol: usize = 0;

        for (idx, location) in locations.iter().enumerate() {
            if idx == 0 {
                line = location.line;
                startcol = location.col;
            }
            endcol = location.col
        }

        (line, startcol, endcol)
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::inlines::{Inline, InlineLiteral, InlineLiteralName};

    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::three_names_and_email(
        "Jane Q. Doe <jane@example.com>",
        ("Jane Q. Doe", "JQD", "Jane", "Q.", "Doe", "jane@example.com")
    )]
    #[case::two_names("John Smith", ("John Smith", "JS", "John", "", "Smith", ""))]
    #[case::one_name("Cher", ("Cher", "C", "Cher", "", "", ""))]
    #[case::underscores("Mary_Ann Smith", ("Mary Ann Smith", "MS", "Mary Ann", "", "Smith", ""))]
    #[case::apostrophe("Seán O'Brien", ("Seán O'Brien", "SO", "Seán", "", "O'Brien", ""))]
    #[case::too_many_names("A B C D", ("A B C D", "A", "A B C D", "", "", ""))]
    fn author_entries(#[case] entry: &str, #[case] expected: (&str, &str, &str, &str, &str, &str)) {
        let (fullname, initials, firstname, middlename, lastname, address) = expected;
        assert_eq!(
            Author::from_line(entry),
            vec![Author {
                fullname: fullname.into(),
                initials: initials.into(),
                firstname: firstname.into(),
                middlename: middlename.into(),
                lastname: lastname.into(),
                address: address.into(),
            }]
        );
    }

    #[test]
    fn several_authors() {
        let authors = Author::from_line("Jane Doe; John Smith <j@s.com>;");
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[1].address, "j@s.com");
    }

    #[rstest]
    #[case::all("v1.2, 2026-01-15: A remark", ("1.2", "2026-01-15", "A remark"))]
    #[case::number("v1.0", ("1.0", "", ""))]
    #[case::date("2026-01-15", ("", "2026-01-15", ""))]
    #[case::number_and_date("v1.0, 2026-01-15", ("1.0", "2026-01-15", ""))]
    #[case::version_word("version 2.1, Jan 2026", ("2.1", "Jan 2026", ""))]
    #[case::date_and_remark("2026-01-15: Fixed", ("", "2026-01-15", "Fixed"))]
    fn revision_lines(#[case] line: &str, #[case] expected: (&str, &str, &str)) {
        let (number, date, remark) = expected;
        assert_eq!(
            Revision::from_line(line),
            Revision {
                number: number.into(),
                date: date.into(),
                remark: remark.into(),
            }
        );
    }

    #[test]
    fn reconcile_locations() {
        let start = vec![Location::new(1, 1, vec![]), Location::new(2, 4, vec![])];
        let other = vec![Location::new(1, 1, vec![]), Location::new(4, 5, vec![])];
        assert_eq!(
            vec![Location::new(1, 1, vec![]), Location::new(4, 5, vec![])],
            Location::reconcile(start, other)
        )
    }

    #[test]
    /// Ensure that any included files are ordered after non-included files even if their lines
    /// and cols are the same
    fn reconcile_locations_files() {
        let included = "foo.txt".to_string();
        let start = vec![Location::new(1, 1, vec![]), Location::new(2, 4, vec![])];
        let other = vec![
            Location::new(1, 1, vec![included.clone()]),
            Location::new(4, 5, vec![included.clone()]),
        ];
        assert_eq!(
            vec![
                Location::new(1, 1, vec![]),
                Location::new(4, 5, vec![included])
            ],
            Location::reconcile(start, other)
        )
    }

    #[test]
    fn header_to_document_id() {
        let mut header = Header::new();
        header.title.push(Inline::InlineLiteral(InlineLiteral::new(
            InlineLiteralName::Text,
            String::from("Foo With Space"),
            vec![],
        )));
        assert_eq!(header.document_id(), "foo_with_space");
    }

    #[test]
    fn header_to_empty_document_id() {
        let header = Header::new();
        assert_eq!(header.document_id(), "");
    }
}
