use std::collections::HashMap;

use serde::Serialize;

use crate::{
    inlines::Inline,
    nodes::{Location, NodeTypes},
};

pub enum _ToFindHomesFor {
    SectionBody,
    NonSectionBlockBody,
}

#[derive(Serialize)]
pub enum Block {
    Section, // sort of a special case but prob needs to be included here
    List(List),
    ListItem(ListItem),
    DList(DList),
    DListItem(DListItem),
    //DiscreteHeading(DiscreteHeading),
    Break(Break),
    BlockMacro(BlockMacro),
    LeafBlock(LeafBlock),
    ParentBlock(ParentBlock), // Admonitions are hiding in here
    BlockMetadata(BlockMetadata),
}

#[derive(Serialize)]
pub struct List {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String,
    variant: ListVariant,
    items: Vec<ListItem>,
    location: Vec<Location>,
}

impl List {
    fn new(variant: ListVariant, marker: String, location: Vec<Location>) -> Self {
        List {
            name: "list".to_string(),
            node_type: NodeTypes::Block,
            marker,
            variant,
            items: vec![],
            location,
        }
    }
}

#[derive(Serialize)]
pub enum ListVariant {
    Callout,
    Ordered,
    Unordered,
}

#[derive(Serialize)]
pub struct ListItem {
    name: String,
    node_type: NodeTypes,
    marker: String,                 // the lexeme with no space
    principal: Option<Vec<Inline>>, // apparently this can also be optional!
    blocks: Option<Vec<Block>>,     // a LI can have subsequent blocks, too
    location: Vec<Location>,
}

impl ListItem {
    fn new(
        marker: String,
        principal: Option<Vec<Inline>>,
        blocks: Option<Vec<Block>>,
        location: Vec<Location>,
    ) -> Self {
        ListItem {
            name: "listItem".to_string(),
            node_type: NodeTypes::Block,
            marker,
            principal,
            blocks,
            location,
        }
    }
}

#[derive(Serialize)]
pub struct DList {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String,
    items: Vec<ListItem>,
    location: Vec<Location>,
}

impl DList {
    fn new(marker: String, location: Vec<Location>) -> Self {
        DList {
            name: "dlist".to_string(),
            node_type: NodeTypes::Block,
            marker,
            items: vec![],
            location,
        }
    }
}

#[derive(Serialize)]
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
    fn new(
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

#[derive(Serialize)]
pub struct Break {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    variant: BreakVariant,
    location: Vec<Location>,
}

#[derive(Serialize)]
pub enum BreakVariant {
    Page,
    Thematic,
}

impl Break {
    fn new(variant: BreakVariant, location: Vec<Location>) -> Self {
        Break {
            name: "break".to_string(),
            node_type: NodeTypes::Block,
            variant,
            location,
        }
    }
}

#[derive(Serialize)]
pub struct BlockMacro {
    name: BlockMacroName,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: String,
    target: String,
    location: Vec<Location>,
}

#[derive(Serialize)]
pub enum BlockMacroName {
    Audio,
    Video,
    Image,
    Toc,
}

impl BlockMacro {
    fn new(name: BlockMacroName, target: String, location: Vec<Location>) -> Self {
        BlockMacro {
            name,
            node_type: NodeTypes::Block,
            form: "macro".to_string(),
            target,
            location,
        }
    }
}

#[derive(Serialize)]
pub struct LeafBlock {
    name: LeafBlockName,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: LeafBlockForm,
    delimiter: Option<String>, // if it's a delimited block, then we provide the delimiter
    inlines: Vec<Inline>,
    //blocks: Vec<Block>, // I'm pretty sure there aren't allowed to have blocks, need to confirm
    location: Vec<Location>,
}

#[derive(Serialize)]
pub enum LeafBlockName {
    Listing,
    Literal,
    Paragraph,
    Pass,
    Stem,
    Verse,
}
#[derive(Serialize)]
pub enum LeafBlockForm {
    Delimited,
    Indented,
    Paragraph,
}

impl LeafBlock {
    fn new(
        name: LeafBlockName,
        form: LeafBlockForm,
        delimiter: Option<String>, // if it's a delimited block, then we provide the delimiter
        inlines: Vec<Inline>,
        location: Vec<Location>,
    ) -> Self {
        LeafBlock {
            name,
            node_type: NodeTypes::Block,
            form,
            delimiter,
            inlines,
            location,
        }
    }
}

#[derive(Serialize)]
pub struct ParentBlock {
    name: ParentBlockName,
    variant: ParetnBlockVarient,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: String,
    delimiter: String, // TK how to handle NOTE:: text...
    blocks: Vec<Block>,
    location: Vec<Location>,
}

#[derive(Serialize)]
pub enum ParentBlockName {
    Admonition,
    Example,
    Sidebar,
    Open,
    Quote,
}

#[derive(Serialize)]
pub enum ParetnBlockVarient {
    Caution,
    Important,
    Note,
    Tip,
    Warning,
}

impl ParentBlock {
    fn new(
        name: ParentBlockName,
        variant: ParetnBlockVarient,
        delimiter: String, // if it's a delimited block, then we provide the delimiter
        blocks: Vec<Block>,
        location: Vec<Location>,
    ) -> Self {
        ParentBlock {
            name,
            variant,
            node_type: NodeTypes::Block,
            form: "delimited".to_string(),
            delimiter,
            blocks,
            location,
        }
    }
}

#[derive(Serialize)]
pub struct BlockMetadata {
    attributes: HashMap<String, String>,
    location: Vec<Location>,
}
