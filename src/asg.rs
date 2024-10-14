use std::collections::HashMap;

use serde::Serialize;

use crate::nodes::{Location, NodeTypes};
use crate::inlines::Inline;
use crate::blocks::Block;


/// Abstract Syntax Graph used to represent an asciidoc document
/// roughly meaning to follow the "official" schema:
/// https://gitlab.eclipse.org/eclipse/asciidoc-lang/asciidoc-lang/-/blob/main/asg/schema.json
#[derive(Serialize)]
pub struct Asg {
    // abstract syntax graph
    pub name: String,                        // is this always "Document?"
    #[serde(rename = "type")]
    pub node_type: NodeTypes,                // is this always "block"
    pub attributes: HashMap<String, String>, // the value can also be a bool; deal with this later
    pub header: Option<Header>,
    pub blocks: Vec<Block>,
    pub location: Vec<Location>, // really a tuple of a "Start" location and an "end" location
}

impl Asg {
    pub fn new() -> Self {
        Asg {
            name: "document".to_string(),
            node_type: NodeTypes::Block,
            attributes: HashMap::new(),
            header: None,
            blocks: vec![],
            location: vec![Location {
                line: 1,
                col: 1,
                file: None,
            }],
        }
    }

    pub fn is_valid(&self) -> bool {
        // more TK
        self.location.len() == 2
    }
}

#[derive(Serialize)]
pub struct Header {
    title: Vec<Inline>,
    authors: Option<Vec<Author>>,
    location: Vec<Location>,
}

#[derive(Serialize)]
pub struct Author {
    fullname: String,
    initials: String,
    firstname: String,
    middlename: String,
    lastname: String,
    address: String,
}

