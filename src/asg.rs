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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::*;
    use crate::inlines::*;

    #[test]
    fn consolidate_footnotes() {
        let mut footnote = Inline::InlineSpan(InlineSpan::new(
            InlineSpanVariant::Footnote,
            InlineSpanForm::Constrained,
            vec![],
        ));
        footnote.push_inline(Inline::InlineLiteral(InlineLiteral::new(
            InlineLiteralName::Text,
            "Foonote text".to_string(),
            vec![],
        )));
        let some_leaf = Block::LeafBlock(LeafBlock::new(
            LeafBlockName::Paragraph,
            LeafBlockForm::Paragraph,
            None,
            vec![],
            vec![footnote],
        ));
        let mut graph = Asg::new();
        graph.document_id = "test".into();
        graph.push_block(some_leaf);
        assert_eq!(graph.blocks.len(), 1);
        // just spot-check that we break them out; the actual logic is checked elsewhere
        graph.consolidate_footnotes();
        assert_eq!(graph.blocks.len(), 2);
        // but also spot-check that we add the document_id, if any
        let Some(Block::LeafBlock(leaf)) = graph.blocks.first() else {
            panic!("Destroyed the block we were only supposed to modify")
        };
        let inlines = leaf.inlines();
        let Some(Inline::InlineSpan(footnoteref)) = inlines.first() else {
            panic!("Missing footnote ref in leaf block")
        };
        let Some(Inline::InlineRef(iref)) = footnoteref.inlines.first() else {
            panic!("Missing footnote ref link")
        };
        assert_eq!(iref.target, "test_footnotedef_1");
    }
}
