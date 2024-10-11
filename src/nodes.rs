use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
pub enum NodeTypes {
    Block,
    Inline,
    String,
}

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
