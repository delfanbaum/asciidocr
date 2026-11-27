//! The graph of a whole document

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::errors::{AsciidocrError, AsgError};
use crate::parser::Parser;
use crate::scanner::Scanner;

use super::blocks::Block;
use super::ids::IdSettings;
use super::inlines::Inline;
use super::{NodeTypes, header::Header, location::Location};

/// Abstract Syntax Graph used to represent an asciidoc document
/// roughly meaning to follow the "official" schema:
/// <https://gitlab.eclipse.org/eclipse/asciidoc-lang/asciidoc-lang/-/blob/main/asg/schema.json>
#[derive(Serialize, Debug)]
pub struct Asg {
    /// This is always "document"
    pub name: String,
    #[serde(rename = "type")]
    /// This is always "block"
    pub node_type: NodeTypes,
    /// The document's attributes (`:name: value`), present when it has a header
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, String>>, // the value can also be a bool; deal with this later
    /// The document header, if it has a title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<Header>,
    #[serde(skip)]
    /// Essentially optional (empty string for convenience), useful when creating things like
    /// footnote references
    document_id: String,
    #[serde(skip)]
    /// Has of all IDs in the document, and the references they point to
    document_id_hash: HashMap<String, Vec<Inline>>,
    /// Document contents (none is no `blocks`, as in the AsciiDoc TCK)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>,
    /// Marks the start and end of document
    pub location: Vec<Location>,
}

impl Default for Asg {
    fn default() -> Self {
        Self::new()
    }
}

impl std::str::FromStr for Asg {
    type Err = AsciidocrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parser::default().parse(Scanner::new(s))?)
    }
}

impl Asg {
    /// An empty document
    pub fn new() -> Self {
        Asg {
            name: "document".to_string(),
            node_type: NodeTypes::Block,
            attributes: None,
            header: None,
            document_id: "".to_string(),
            document_id_hash: HashMap::new(),
            blocks: vec![],
            location: vec![Location::default()],
        }
    }

    /// Standardizes the graph into the "official" ASG format. Since we keep an intermediate
    /// representation that differs slightly (e.g., see footnotes), we need to do this ahead of
    /// JSON or (eventually) "asciidoctor-style" HTML output.
    pub(crate) fn consolidate(&mut self) {
        self.consolidate_locations();
        self.consolidate_xrefs();
    }

    pub(crate) fn add_header(&mut self, header: Header, doc_attributes: HashMap<String, String>) {
        // add document_id if there is one
        self.document_id = header.document_id();
        self.header = Some(header);
        // always add attributes if there is a header, even if empty
        self.attributes = Some(doc_attributes);
    }

    /// Adds a block (tree) to the "root" of the document
    pub(crate) fn push_block(&mut self, mut block: Block) -> Result<(), AsgError> {
        block.consolidate_locations();
        self.document_id_hash.extend(block.id_hashes());
        if block.is_section() {
            if let Some(possible_section) = self.blocks.last_mut() {
                // only a (higher-level) section can contain this section; other blocks that take
                // children (sidebars, examples, tables, etc.) are complete once they get here
                if possible_section.is_section() && possible_section.takes_block_of_type(&block) {
                    possible_section.push_block(block)?;
                } else {
                    self.blocks.push(block);
                }
            } else {
                self.blocks.push(block);
            }
        } else {
            self.blocks.push(block)
        }
        Ok(())
    }

    /// Generates IDs for the sections that weren't given one, as the document's attributes
    /// (`sectids`, `idprefix`, `idseparator`) ask, so that the backends can use them, and cross
    /// references to them find the section's title. They aren't part of the JSON (TCK) output.
    pub(crate) fn generate_section_ids(&mut self, attributes: &HashMap<String, String>) {
        let settings = IdSettings::from_attributes(attributes);
        let mut taken = HashSet::new();
        let mut titles = HashMap::new();
        for block in self.blocks.iter_mut() {
            block.generate_section_ids(&settings, &mut taken, &mut titles);
        }
        // (unique, so these don't replace the explicit IDs)
        for (id, title) in titles {
            self.document_id_hash.entry(id).or_insert(title);
        }
    }

    /// Consolidates location information about the tree
    pub(crate) fn consolidate_locations(&mut self) {
        if let Some(last_block) = self.blocks.last_mut() {
            self.location = Location::reconcile(self.location.clone(), last_block.locations())
        } else {
            // if there is only a header, for example, or nothing
            if let Some(header) = &self.header {
                self.location = Location::reconcile(self.location.clone(), header.location.clone())
            }
        }
    }

    /// If possible, puts the title text of a given referenced element into the xref Inline so that
    /// the relevant text is displayed as a part of the link; if the xref is missing or not a
    /// title, notify the user
    fn consolidate_xrefs(&mut self) {
        for block in self.blocks.iter_mut() {
            for inline in block.inlines_mut() {
                // easier to push the matching down to the Inline enum
                inline.attempt_xref_standardization(&self.document_id_hash);
            }
        }
    }

    /// Gathers all the "literal" text in a given document graph (i.e., without styling, etc.)
    /// (Largely a testing/verification function)
    pub(crate) fn all_text(&self) -> String {
        let mut graph_text = String::new();
        for block in self.blocks.iter() {
            graph_text.push_str(&block.block_text())
        }
        graph_text
    }

    /// The number of words in the document's text, not counting markup or comments
    pub fn word_count(&self) -> usize {
        self.all_text().split_whitespace().count()
    }
}
