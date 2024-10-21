use std::{collections::HashMap, fmt::Display};

use serde::Serialize;

use crate::{
    inlines::Inline,
    nodes::{Location, NodeTypes},
};

pub enum _ToFindHomesFor {}

#[derive(Serialize, PartialEq, Debug)]
#[serde(untagged)]
pub enum Block {
    Section(Section), // sort of a special case but prob needs to be included here
    SectionBody,
    NonSectionBlockBody,
    List(List),
    //ListItem(ListItem), // not sure we ever deal with this as a "block"
    DList(DList),
    //DListItem(DListItem), // ditto listitem
    DiscreteHeading, // not handled currently
    Break(Break),
    BlockMacro(BlockMacro),
    LeafBlock(LeafBlock),
    ParentBlock(ParentBlock), // Admonitions are hiding in here
                              //BlockMetadata(BlockMetadata),
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Block::Section(_) => write!(f, "Section"),
            Block::SectionBody => write!(f, "SectionBody"),
            Block::NonSectionBlockBody => write!(f, "NonSectionBlockBody"),
            Block::List(_) => write!(f, "List"),
            //Block::ListItem(_) => write!(f, "ListItem"),
            Block::DList(_) => write!(f, "DList"),
            //Block::DListItem(_) => write!(f, "DListItem"),
            Block::DiscreteHeading => write!(f, "DiscreteHeading"),
            Block::Break(_) => write!(f, "Break"),
            Block::BlockMacro(_) => write!(f, "BlockMacro"),
            Block::LeafBlock(_) => write!(f, "LeafBlock"),
            Block::ParentBlock(_) => write!(f, "ParentBlock"),
            //Block::BlockMetadata(_) => write!(f, "BlockMetadata"),
        }
    }
}

impl Block {
    pub fn push_block(&mut self, block: Block) {
        match self {
            Block::Section(section) => section.blocks.push(block),
            _ => panic!("push_block not implemented for {}", self),
        }
    }

    pub fn push_inline(&mut self, inline: Inline) {
        match self {
            Block::Section(section) => section.title.push(inline),
            Block::LeafBlock(block) => block.inlines.push(inline),
            _ => panic!("push_block not implemented for {}", self),
        }
    }

    pub fn consolidate_inlines(&mut self) {
        match self {
            Block::Section(section) => {
                // consolidate title
                let mut consolidated: Vec<Inline> = vec![];
                while let Some(mut inline) = section.title.pop() {
                    if inline.is_literal() {
                        if let Some(prev_inline) = consolidated.last_mut() {
                            if prev_inline.is_literal() {
                                let extracted_inline = inline.extract_literal();
                                prev_inline.prepend_literal(extracted_inline)
                            }
                        } else {
                            consolidated.push(inline);
                        }
                    } else {
                        consolidated.push(inline);
                    }
                }
                // reverse the list
                consolidated.reverse();
                // replace the title
                section.title = consolidated;

                // consolidate everything else
                for block in section.blocks.iter_mut() {
                    block.consolidate_inlines()
                }
            }
            Block::LeafBlock(block) => {
                let mut consolidated: Vec<Inline> = vec![];
                while let Some(mut inline) = block.inlines.pop() {
                    if inline.is_literal() {
                        if let Some(prev_inline) = consolidated.last_mut() {
                            if prev_inline.is_literal() {
                                let extracted_inline = inline.extract_literal();
                                prev_inline.prepend_literal(extracted_inline)
                            }
                        } else {
                            consolidated.push(inline);
                        }
                    } else {
                        consolidated.push(inline);
                    }
                }
                // reverse the list
                consolidated.reverse();
                // replace the inlines
                block.inlines = consolidated;
            }
            _ => {}
        }
    }

    pub fn can_be_parent(&self) -> bool {
        match self {
            Block::Section(_) => true,
            _ => false,
        }
    }
    pub fn is_section(&self) -> bool {
        match self {
            Block::Section(_) => true,
            _ => false,
        }
    }

    pub fn has_blocks(&self) -> bool {
        match self {
            Block::Section(section) => !section.blocks.is_empty(),
            Block::LeafBlock(_) => false,
            _ => true,
        }
    }

    pub fn create_id(&mut self) {
        match self {
            Block::Section(section) => {
                if section.id == "".to_string() {
                    let mut id = String::new();
                    for inline in &section.title {
                        id.push_str(&inline.extract_values_to_string())
                    }
                    id = id.replace(" ", "-");
                    section.id = id
                }
            }
            _ => {}
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Section {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    pub id: String,
    title: Vec<Inline>,
    reftext: Vec<Inline>,
    metadata: Option<BlockMetadata>,
    pub level: usize,
    blocks: Vec<Block>,
    location: Vec<Location>,
}

impl PartialEq for Section {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Section {
    // generated as a result of a heading
    pub fn new(id: String, level: usize, first_location: Location) -> Self {
        Section {
            name: "section".to_string(),
            node_type: NodeTypes::Block,
            id,
            title: vec![],   // added later
            reftext: vec![], // added later
            metadata: None,
            level,
            blocks: vec![],
            location: vec![first_location],
        }
    }
}

#[derive(Serialize, Debug)]
pub struct List {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String,
    variant: ListVariant,
    items: Vec<ListItem>,
    location: Vec<Location>,
}

impl PartialEq for List {
    fn eq(&self, other: &Self) -> bool {
        self.variant == other.variant
    }
}

impl List {
    pub fn new(variant: ListVariant, marker: String, location: Vec<Location>) -> Self {
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

#[derive(Serialize, PartialEq, Eq, Debug)]
pub enum ListVariant {
    Callout,
    Ordered,
    Unordered,
}

#[derive(Serialize, Debug)]
pub struct ListItem {
    name: String,
    node_type: NodeTypes,
    marker: String,                 // the lexeme with no space
    principal: Option<Vec<Inline>>, // apparently this can also be optional!
    blocks: Option<Vec<Block>>,     // a LI can have subsequent blocks, too
    location: Vec<Location>,
}

impl ListItem {
    pub fn new(
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

#[derive(Serialize, Debug)]
pub struct DList {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String,
    items: Vec<ListItem>,
    location: Vec<Location>,
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

#[derive(Serialize, Debug)]
pub struct Break {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    variant: BreakVariant,
    location: Vec<Location>,
}

impl PartialEq for Break {
    fn eq(&self, other: &Self) -> bool {
        self.variant == other.variant
    }
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub enum BreakVariant {
    Page,
    Thematic,
}

impl Break {
    pub fn new(variant: BreakVariant, location: Vec<Location>) -> Self {
        Break {
            name: "break".to_string(),
            node_type: NodeTypes::Block,
            variant,
            location,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct BlockMacro {
    name: BlockMacroName,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: String,
    target: String,
    location: Vec<Location>,
}

impl PartialEq for BlockMacro {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Serialize, PartialEq, Debug)]
pub enum BlockMacroName {
    Audio,
    Video,
    Image,
    Toc,
}

impl BlockMacro {
    pub fn new(name: BlockMacroName, target: String, location: Vec<Location>) -> Self {
        BlockMacro {
            name,
            node_type: NodeTypes::Block,
            form: "macro".to_string(),
            target,
            location,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct LeafBlock {
    pub name: LeafBlockName,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: LeafBlockForm,
    delimiter: Option<String>, // if it's a delimited block, then we provide the delimiter
    inlines: Vec<Inline>,
    //blocks: Vec<Block>, // I'm pretty sure there aren't allowed to have blocks, need to confirm
    location: Vec<Location>,
}

#[derive(Serialize, Debug, PartialEq)]
pub enum LeafBlockName {
    Listing,
    Literal, // TK not handling now
    Paragraph,
    Pass,
    Stem,  // TK not handling now
    Verse, // TK need to figure handling for quotes
}
#[derive(Serialize, Debug)]
pub enum LeafBlockForm {
    Delimited,
    Indented,
    Paragraph,
}

impl PartialEq for LeafBlock {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl LeafBlock {
    pub fn new(
        // note that the locations must be calculated later
        name: LeafBlockName,
        form: LeafBlockForm,
        delimiter: Option<String>, // if it's a delimited block, then we provide the delimiter
        location: Vec<Location>,
        inlines: Vec<Inline>,
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
    pub fn new_listing(delimiter: Option<String>, start_location: Location) -> Self {
        Self::new(
            LeafBlockName::Listing,
            LeafBlockForm::Delimited,
            delimiter,
            vec![start_location],
            vec![],
        )
    }
    pub fn new_pass(
        // note that the locations must be calculated later
        delimiter: Option<String>, // if it's a delimited block, then we provide the delimiter
        start_location: Location,
    ) -> Self {
        Self::new(
            LeafBlockName::Pass,
            LeafBlockForm::Delimited,
            delimiter,
            vec![start_location],
            vec![],
        )
    }
}

#[derive(Serialize, Debug)]
pub struct ParentBlock {
    name: ParentBlockName,
    variant: Option<ParentBlockVarient>,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: String,
    delimiter: String, // TK how to handle NOTE:: text...
    blocks: Vec<Block>,
    location: Vec<Location>,
}

impl PartialEq for ParentBlock {
    fn eq(&self, other: &Self) -> bool {
        if let Some(variant) = &self.variant {
            if let Some(other_variant) = &other.variant {
                variant == other_variant && &self.name == &other.name
            } else {
                false
            }
        } else {
            self.name == other.name
        }
    }
}

#[derive(Serialize, Debug, PartialEq)]
pub enum ParentBlockName {
    Admonition,
    Example,
    Sidebar,
    Open,
    Quote,
}

#[derive(Serialize, Debug, PartialEq)]
pub enum ParentBlockVarient {
    Caution,
    Important,
    Note,
    Tip,
    Warning,
}

impl ParentBlock {
    pub fn new(
        name: ParentBlockName,
        variant: Option<ParentBlockVarient>,
        delimiter: String, // if it's a delimited block, then we provide the delimiter
        blocks: Vec<Block>,
    ) -> Self {
        ParentBlock {
            name,
            variant,
            node_type: NodeTypes::Block,
            form: "delimited".to_string(),
            delimiter,
            blocks,
            location: vec![],
        }
    }
}

#[derive(Serialize, Debug)]
pub struct BlockMetadata {
    attributes: HashMap<String, String>,
    location: Vec<Location>,
}
