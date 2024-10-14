use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
pub enum NodeTypes {
    Block,
    Inline,
    String,
}

/// A "location" pertaining to a given document object, usually the start or end of something
#[derive(Serialize)]
pub struct Location {
    pub line: usize, // 1-indexed
    pub col: usize,  // 1-indexed
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
