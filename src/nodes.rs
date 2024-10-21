use serde::Serialize;

use crate::inlines::Inline;

#[derive(Clone, Copy, Serialize, Debug)]
pub enum NodeTypes {
    Block,
    Inline,
    String,
}

/// Struct containing document header information
#[derive(Serialize, Clone, Debug)]
pub struct Header {
    pub title: Vec<Inline>,
    pub authors: Option<Vec<Author>>,
    pub location: Vec<Location>,
}

impl Header {
    pub fn new() -> Self {
        Header {
            title: vec![],
            authors: None,
            location: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.title.is_empty() && self.authors.is_none()
    }
}

/// Struct containing document author information
#[derive(Serialize, Clone, Debug)]
pub struct Author {
    fullname: String,
    initials: String,
    firstname: String,
    middlename: String,
    lastname: String,
    address: String,
}

/// A "location" pertaining to a given document object, usually the start or end of something
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Location {
    pub line: usize, // 1-indexed
    pub col: usize,  // 1-indexed
                     //#[serde(skip_serializing_if = "Option::is_none")]
                     //pub file: Option<Vec<String>>, // I *think* this is for includes, though we're not going to handle
                     // those yet
}

impl Default for Location {
    fn default() -> Self {
        Location {
            line: 1,
            col: 1,
            //file: None,
        }
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.line == other.line {
            self.col.cmp(&other.col)
        } else {
            self.line.cmp(&other.line)
        }
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.line == other.line {
            Some(self.col.cmp(&other.col))
        } else {
            Some(self.line.cmp(&other.line))
        }
    }
}

impl Location {
    pub fn new(line: usize, col: usize) -> Self {
        Location {
            line,
            col,
            //file: None,
        }
    } // handle file later

    pub fn reconcile(mut start: Vec<Location>, other: Vec<Location>) -> Vec<Location> {
        start.extend(other);
        start.sort();
        // remove the middle
        start.drain(1..start.len()-1);
        start
    }
}

#[cfg(test)]
mod tests {
    use super::Location;

    #[test]
    fn reconcile_locations() {
        let start = vec![Location::new(1, 1), Location::new(2, 4)];
        let other = vec![Location::new(1, 1), Location::new(4, 5)];
        assert_eq!(
            vec![Location::new(1, 1), Location::new(4, 5)],
            Location::reconcile(start, other)
        )
    }
}
