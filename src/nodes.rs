use serde::Serialize;

use crate::inlines::Inline;

#[derive(Clone, Copy, Serialize)]
pub enum NodeTypes {
    Block,
    Inline,
    String,
}

/// Struct containing document header information
#[derive(Serialize)]
pub struct Header {
    title: Vec<Inline>,
    authors: Option<Vec<Author>>,
    location: Vec<Location>,
}

/// Struct containing document author information
#[derive(Serialize)]
pub struct Author {
    fullname: String,
    initials: String,
    firstname: String,
    middlename: String,
    lastname: String,
    address: String,
}

/// A "location" pertaining to a given document object, usually the start or end of something
#[derive(Serialize)]
pub struct Location {
    pub line: usize, // 1-indexed
    pub col: usize,  // 1-indexed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<Vec<String>>, // I *think* this is for includes, though we're not going to handle
                                   // those yet
}

impl Default for Location {
    fn default() -> Self {
        Location {
            line: 1,
            col: 1,
            file: None,
        }
    }
}

impl Location {
    pub fn new(line: usize, col: usize) -> Self {
        Location {
            line,
            col,
            file: None,
        }
    } // handle file later
}
