use crate::graph::{
    blocks::{Block, LeafBlock},
    inlines::Inline,
    nodes::{Location, NodeTypes},
};
use crate::tokens::Token;
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct List {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String,
    pub variant: ListVariant,
    pub items: Vec<Block>,
    pub location: Vec<Location>,
}

impl PartialEq for List {
    fn eq(&self, other: &Self) -> bool {
        self.variant == other.variant
    }
}

impl List {
    pub fn new(variant: ListVariant, location: Vec<Location>) -> Self {
        let mut list_marker = String::new();
        match variant {
            ListVariant::Unordered => list_marker.push('*'),
            ListVariant::Ordered => list_marker.push('.'),
            ListVariant::Callout => todo!(),
        }

        List {
            name: "list".to_string(),
            node_type: NodeTypes::Block,
            marker: list_marker,
            variant,
            items: vec![],
            location,
        }
    }

    pub fn add_item(&mut self, item: Block) {
        self.items.push(item)
    }
}

#[derive(Serialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ListVariant {
    Callout,
    Ordered,
    Unordered,
}

#[derive(Serialize, Clone, Debug)]
pub struct ListItem {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    pub marker: String,         // the lexeme with no space
    pub principal: Vec<Inline>, // apparently this can also be optional!
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>, // a LI can have subsequent blocks, too
    pub location: Vec<Location>,
}

impl PartialEq for ListItem {
    fn eq(&self, other: &Self) -> bool {
        self.marker == other.marker
    }
}

impl ListItem {
    pub fn new(marker: String, location: Vec<Location>) -> Self {
        let trimmed_mark = marker.trim().to_string();
        ListItem {
            name: "listItem".to_string(),
            node_type: NodeTypes::Block,
            marker: trimmed_mark,
            principal: vec![],
            blocks: vec![],
            location,
        }
    }

    pub fn add_inline(&mut self, inline: Inline) {
        self.principal.push(inline)
    }

    pub fn principal(&self) -> Vec<Inline> {
        self.principal.clone()
    }

    pub fn blocks(&self) -> Vec<Block> {
        self.blocks.clone()
    }

    pub fn extract_footnotes(&mut self, count: usize, document_id: &str) -> Vec<Block> {
        let mut local_count = count;
        let mut extracted: Vec<Block> = Vec::new();
        for idx in 0..self.principal.len() {
            if self.principal[idx].is_footnote() {
                local_count += 1;
                let inline_span = self.principal.remove(idx);
                let Inline::InlineSpan(mut footnote) = inline_span else {
                    panic!("Bad is_footnote match")
                };
                // deconstruct it
                let (definition_id, replacement_span, footnote_contents) =
                    footnote.deconstruct_footnote(local_count, document_id);
                // add the relevant stuff to the return
                extracted.push(Block::LeafBlock(
                    LeafBlock::new_footnote_def_from_id_and_inlines(
                        definition_id,
                        footnote_contents,
                    ),
                ));
                // put the reference back where the span was
                self.principal.insert(idx, replacement_span);
            }
        }
        for child in self.blocks.iter_mut() {
            let child_footnoes = child.extract_footnote_definitions(local_count, document_id);
            local_count += child_footnoes.len();
            extracted.extend(child_footnoes);
        }
        extracted
    }
}

/// Functions essentially the same as a list
#[derive(Serialize, Clone, Debug)]
pub struct DList {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String,
    pub items: Vec<Block>,
    pub location: Vec<Location>,
}

impl PartialEq for DList {
    // all dlists are dlists
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl DList {
    pub fn new(location: Vec<Location>) -> Self {
        DList {
            name: "dlist".to_string(),
            node_type: NodeTypes::Block,
            marker: "::".to_string(),
            items: vec![],
            location,
        }
    }
    pub fn add_item(&mut self, item: Block) {
        self.items.push(item)
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct DListItem {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String, // the lexeme with no space
    pub terms: Vec<Inline>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub principal: Vec<Inline>, // apparently this can also be optional!
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>, // a LI can have subsequent blocks, too
    pub location: Vec<Location>,
}

impl PartialEq for DListItem {
    // all dlistitems are dlist items
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl DListItem {
    pub fn new_from_token(token: Token) -> Self {
        DListItem {
            name: "dlistItem".to_string(),
            node_type: NodeTypes::Block,
            marker: "::".to_string(),
            terms: vec![],
            principal: vec![],
            blocks: vec![],
            location: token.locations(),
        }
    }

    /// adds inlines to the "terms", and reconciling the locations.
    pub fn push_term(&mut self, inline: Inline) {
        self.location = Location::reconcile(self.location.clone(), inline.locations().clone());
        self.terms.push(inline)
    }

    pub fn add_inline(&mut self, inline: Inline) {
        self.principal.push(inline)
    }

    pub fn locations(&self) -> Vec<Location> {
        self.location.clone()
    }

    pub fn extract_footnotes(&mut self, count: usize, document_id: &str) -> Vec<Block> {
        let mut local_count = count;
        let mut extracted: Vec<Block> = Vec::new();
        for idx in 0..self.principal.len() {
            if self.principal[idx].is_footnote() {
                local_count += 1;
                let inline_span = self.principal.remove(idx);
                let Inline::InlineSpan(mut footnote) = inline_span else {
                    panic!("Bad is_footnote match")
                };
                // deconstruct it
                let (definition_id, replacement_span, footnote_contents) =
                    footnote.deconstruct_footnote(local_count, document_id);
                // add the relevant stuff to the return
                extracted.push(Block::LeafBlock(
                    LeafBlock::new_footnote_def_from_id_and_inlines(
                        definition_id,
                        footnote_contents,
                    ),
                ));
                // put the reference back where the span was
                self.principal.insert(idx, replacement_span);
            }
        }
        for child in self.blocks.iter_mut() {
            let child_footnoes = child.extract_footnote_definitions(local_count, document_id);
            local_count += child_footnoes.len();
            extracted.extend(child_footnoes);
        }
        extracted
    }
}
