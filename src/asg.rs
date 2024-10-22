use std::collections::HashMap;

use serde::Serialize;

use crate::blocks::Block;
use crate::nodes::{Header, Location, NodeTypes};

/// Abstract Syntax Graph used to represent an asciidoc document
/// roughly meaning to follow the "official" schema:
/// https://gitlab.eclipse.org/eclipse/asciidoc-lang/asciidoc-lang/-/blob/main/asg/schema.json
#[derive(Serialize)]
pub struct Asg {
    // abstract syntax graph
    pub name: String, // is this always "Document?"
    #[serde(rename = "type")]
    pub node_type: NodeTypes, // is this always "block"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, String>>, // the value can also be a bool; deal with this later
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<Header>,
    pub blocks: Vec<Block>,
    pub location: Vec<Location>, // really a tuple of a "Start" location and an "end" location
}

impl Asg {
    pub fn new() -> Self {
        Asg {
            name: "document".to_string(),
            node_type: NodeTypes::Block,
            attributes: None,
            header: None,
            blocks: vec![],
            location: vec![Location::default()],
        }
    }

    pub fn add_header(&mut self, header: Header) {
        self.attributes = Some(HashMap::new());
        self.header = Some(header);
        // add the attributes from the header... later
    }

    /// Adds a block (tree) to the "root" of the document
    pub fn push_block(&mut self, mut block: Block) {
        block.consolidate_locations();
        //block.trim_literals();
        self.blocks.push(block)
    }

    /// Consolidates location (and, later, other) information about the tree
    pub fn consolidate(&mut self) {
        if let Some(last_block) = self.blocks.last_mut() {
            self.location = Location::reconcile(self.location.clone(), last_block.locations())
        }
    }

    pub fn is_valid(&self) -> bool {
        // more TK
        self.location.len() == 2
    }
}
