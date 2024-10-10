use serde::Serialize;


#[derive(Serialize)]
pub enum Block {}


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
