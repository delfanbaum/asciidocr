use std::collections::HashMap;

use serde::Serialize;

use crate::blocks::{Block, ParentBlock};
use crate::nodes::{Header, Location, NodeTypes};

/// Abstract Syntax Graph used to represent an asciidoc document
/// roughly meaning to follow the "official" schema:
/// https://gitlab.eclipse.org/eclipse/asciidoc-lang/asciidoc-lang/-/blob/main/asg/schema.json
#[derive(Serialize, Debug)]
pub struct Asg {
    // abstract syntax graph
    pub name: String, // is this always "Document?"
    #[serde(rename = "type")]
    pub node_type: NodeTypes, // is this always "block"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, String>>, // the value can also be a bool; deal with this later
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<Header>,
    #[serde(skip)]
    document_id: String,
    pub blocks: Vec<Block>,
    pub location: Vec<Location>, // really a tuple of a "Start" location and an "end" location
}

impl Default for Asg {
    fn default() -> Self {
        Self::new()
    }
}

impl Asg {
    pub fn new() -> Self {
        Asg {
            name: "document".to_string(),
            node_type: NodeTypes::Block,
            attributes: None,
            header: None,
            document_id: "".to_string(),
            blocks: vec![],
            location: vec![Location::default()],
        }
    }

    pub fn add_header(&mut self, header: Header, doc_attributes: HashMap<String, String>) {
        // add document_id if there is one
        self.document_id = header.document_id();
        self.header = Some(header);
        // always add attributes if there is a header, even if empty
        self.attributes = Some(doc_attributes);
    }

    /// Adds a block (tree) to the "root" of the document
    pub fn push_block(&mut self, mut block: Block) {
        block.consolidate_locations();
        self.blocks.push(block)
    }

    /// Consolidates location (and, later, other) information about the tree
    pub fn consolidate(&mut self) {
        if let Some(last_block) = self.blocks.last_mut() {
            self.location = Location::reconcile(self.location.clone(), last_block.locations())
        }
    }

    /// Standardizes the graph into the "official" ASG format. Since we keep an intermediate
    /// representation that differs slightly (e.g., see footnotes), we need to do this ahead of
    /// JSON or (eventually) "asciidoctor-style" HTML output.
    pub fn standardize(&mut self) {
        // TODO run functions
        self.consolidate_footnotes();
    }

    //<div class="paragraph">
    //<p>This is a test.<sup class="footnote">[<a id="_footnoteref_1" class="footnote" href="#_footnotedef_1" title="View footnote.">1</a>]</sup></p>
    //</div>
    //<div class="paragraph">
    //<p>This is a second test.<sup class="footnote">[<a id="_footnoteref_2" class="footnote" href="#_footnotedef_2" title="View footnote.">2</a>]</sup></p>
    //</div>
    //</div>
    //<div id="footnotes">
    //<hr>
    //<div class="footnote" id="_footnotedef_1">
    //<a href="#_footnoteref_1">1</a>. First footnote.
    //</div>
    //<div class="footnote" id="_footnotedef_2">
    //<a href="#_footnoteref_2">2</a>. Second footnote.
    //</div>
    //</div>
    fn consolidate_footnotes(&mut self) {
        // Until the spec says otherwise, put footnote definitions in leaf blocks
        let mut footnote_defs: Vec<Block> = vec![];
        for block in self.blocks.iter_mut() {
            footnote_defs
                .extend(block.extract_footnote_definitions(footnote_defs.len(), &self.document_id));
        }
        // create a parent block to hold the footnote definitions
        self.push_block(Block::ParentBlock(ParentBlock::new_footnotes_container(
            footnote_defs,
        )))
    }

    /// Gathers all the "literal" text in a given document graph (i.e., without styling, etc.)
    /// (Largely a testing/verification function)
    pub fn all_text(&self) -> String {
        let mut graph_text = String::new();
        for block in self.blocks.iter() {
            graph_text.push_str(&block.block_text())
        }
        graph_text
    }
}
