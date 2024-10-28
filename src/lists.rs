use crate::{
    blocks::Block,
    inlines::Inline,
    nodes::{Location, NodeTypes},
};
use serde::Serialize;

#[derive(Serialize, Debug)]
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

#[derive(Serialize, Debug)]
pub struct ListItem {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    pub marker: String,             // the lexeme with no space
    pub principal: Vec<Inline>,     // apparently this can also be optional!
    #[serde(skip_serializing_if="Vec::is_empty")]
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
}

#[derive(Serialize, Debug)]
pub struct DList {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String,
    items: Vec<ListItem>,
    pub location: Vec<Location>,
}

impl PartialEq for DList {
    // all dlists are dlists
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl DList {
    pub fn new(marker: String, location: Vec<Location>) -> Self {
        DList {
            name: "dlist".to_string(),
            node_type: NodeTypes::Block,
            marker,
            items: vec![],
            location,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct DListItem {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String, // the lexeme with no space
    terms: Vec<Inline>,
    principal: Option<Vec<Inline>>, // apparently this can also be optional!
    blocks: Option<Vec<Block>>,     // a LI can have subsequent blocks, too
    location: Vec<Location>,
}

impl DListItem {
    pub fn new(
        marker: String,
        terms: Vec<Inline>,
        principal: Option<Vec<Inline>>,
        blocks: Option<Vec<Block>>,
        location: Vec<Location>,
    ) -> Self {
        DListItem {
            name: "dlistItem".to_string(),
            node_type: NodeTypes::Block,
            marker,
            terms,
            principal,
            blocks,
            location,
        }
    }
}
