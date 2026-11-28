//! Where things are in the source

use serde::Serialize;

/// A "location" pertaining to a given document object, usually the start or end of something
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Location {
    /// The line, counting from 1
    pub line: usize, // 1-indexed
    /// The column, counting from 1
    pub col: usize, // 1-indexed
    /// The files that were included to get here, outermost first; empty for the document itself
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
    pub(crate) fn new(line: usize, col: usize, file: Vec<String>) -> Self {
        Location { line, col, file }
    }

    /// The column, counting from 1
    pub fn col(&self) -> usize {
        self.col
    }

    pub(crate) fn reconcile(mut start: Vec<Location>, other: Vec<Location>) -> Vec<Location> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
