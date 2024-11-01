use core::panic;
use std::{collections::HashMap, fmt::Display};

use serde::Serialize;

use crate::{
    inlines::Inline,
    lists::{DList, List, ListItem, ListVariant},
    nodes::{Location, NodeTypes},
    tokens::{Token, TokenType},
};

pub enum _ToFindHomesFor {}

/// Blocks enum, containing any tree blocks
#[derive(Serialize, PartialEq, Debug)]
#[serde(untagged)]
pub enum Block {
    Section(Section), // sort of a special case but prob needs to be included here
    SectionBody,
    NonSectionBlockBody,
    List(List),
    ListItem(ListItem),
    DList(DList),
    DiscreteHeading, // not handled currently
    Break(Break),
    BlockMacro(BlockMacro),
    LeafBlock(LeafBlock),
    ParentBlock(ParentBlock), // Admonitions are hiding in here
    BlockMetadata(BlockMetadata),
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Block::Section(_) => write!(f, "Section"),
            Block::SectionBody => write!(f, "SectionBody"),
            Block::NonSectionBlockBody => write!(f, "NonSectionBlockBody"),
            Block::List(_) => write!(f, "List"),
            Block::ListItem(_) => write!(f, "ListItem"),
            Block::DList(_) => write!(f, "DList"),
            //Block::DListItem(_) => write!(f, "DListItem"),
            Block::DiscreteHeading => write!(f, "DiscreteHeading"),
            Block::Break(_) => write!(f, "Break"),
            Block::BlockMacro(_) => write!(f, "BlockMacro"),
            Block::LeafBlock(_) => write!(f, "LeafBlock"),
            Block::ParentBlock(_) => write!(f, "ParentBlock"),
            Block::BlockMetadata(_) => write!(f, "BlockMetadata"),
        }
    }
}

impl Block {
    pub fn last_inline(&mut self) -> Option<&mut Inline> {
        match self {
            Block::LeafBlock(block) => Some(block.inlines.last_mut()?),
            _ => None,
        }
    }

    pub fn push_block(&mut self, block: Block) {
        match self {
            Block::Section(section) => section.blocks.push(block),
            Block::List(list) => list.add_item(block),
            Block::ListItem(list_item) => list_item.blocks.push(block),
            Block::ParentBlock(parent_block) => parent_block.blocks.push(block),
            _ => panic!("push_block not implemented for {}", self),
        }
        self.consolidate_locations();
    }

    pub fn takes_inlines(&self) -> bool {
        matches!(
            self,
            Block::Section(_) | Block::LeafBlock(_) | Block::ListItem(_)
        )
    }

    pub fn push_inline(&mut self, inline: Inline) {
        match self {
            Block::Section(section) => section.inlines.push(inline),
            Block::LeafBlock(block) => block.inlines.push(inline),
            Block::ListItem(list_item) => list_item.add_inline(inline),
            _ => panic!("push_block not implemented for {}", self),
        }
    }

    pub fn consolidate_locations(&mut self) {
        match self {
            Block::Section(block) => {
                if let Some(last_inline) = block.blocks.last() {
                    block.location =
                        Location::reconcile(block.location.clone(), last_inline.locations())
                }
            }
            Block::LeafBlock(block) => {
                if let Some(last_inline) = block.inlines.last() {
                    block.location =
                        Location::reconcile(block.location.clone(), last_inline.locations())
                }
            }
            Block::List(list) => {
                for block in &mut list.items {
                    block.consolidate_locations()
                }
                if let Some(last_block) = list.items.last() {
                    list.location =
                        Location::reconcile(list.location.clone(), last_block.locations())
                }
            }
            Block::ListItem(block) => {
                if let Some(last_block) = block.blocks.last() {
                    block.location =
                        Location::reconcile(block.location.clone(), last_block.locations())
                } else if let Some(last_inline) = block.principal.last() {
                    block.location =
                        Location::reconcile(block.location.clone(), last_inline.locations())
                }
            }
            Block::ParentBlock(block) => {
                if let Some(last_inline) = block.blocks.last() {
                    block.location =
                        Location::reconcile(block.location.clone(), last_inline.locations())
                }
            }

            _ => todo!(),
        }
    }

    pub fn can_be_parent(&self) -> bool {
        matches!(
            self,
            Block::Section(_) | Block::ParentBlock(_) | Block::List(_) | Block::ListItem(_)
        )
    }

    pub fn is_section(&self) -> bool {
        matches!(self, Block::Section(_))
    }

    pub fn level_check(&self) -> Option<usize> {
        match self {
            Block::Section(section) => Some(section.level),
            _ => None,
        }
    }

    pub fn list_type(&self) -> Option<ListVariant> {
        match self {
            Block::List(list) => Some(list.variant.clone()),
            _ => None,
        }
    }

    pub fn is_list_item(&self) -> bool {
        matches!(self, Block::ListItem(_))
    }

    pub fn is_ordered_list_item(&self) -> bool {
        match self {
            Block::ListItem(list) => list.marker == *".",
            _ => false,
        }
    }

    pub fn is_unordered_list_item(&self) -> bool {
        match self {
            Block::ListItem(list) => list.marker == *"*",
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

    /// helper when we need to move child blocks from one Block to another
    pub fn extract_blocks(&mut self) -> Vec<Block> {
        match self {
            Block::List(ref mut list) => list.items.drain(..).collect(),
            _ => {
                let v: Vec<Block> = Vec::new();
                v
            }
        }
    }

    pub fn create_id(&mut self) {
        if let Block::Section(section) = self {
            if section.id == *"" {
                let mut id = String::new();
                for inline in &section.inlines {
                    id.push_str(&inline.extract_values_to_string())
                }
                id = id.replace(" ", "-");
                section.id = id
            }
        }
    }

    pub fn locations(&self) -> Vec<Location> {
        match self {
            Block::Section(block) => block.location.clone(),
            Block::SectionBody => vec![],
            Block::NonSectionBlockBody => vec![],
            Block::List(block) => block.location.clone(),
            Block::ListItem(block) => block.location.clone(),
            Block::DList(block) => block.location.clone(),
            //Block::DListItem(_) => write!(f, "DListItem"),
            Block::DiscreteHeading => vec![],
            Block::Break(block) => block.location.clone(),
            Block::BlockMacro(block) => block.location.clone(),
            Block::LeafBlock(block) => block.location.clone(),
            Block::ParentBlock(block) => block.location.clone(),
            Block::BlockMetadata(block) => block.location.clone(),
        }
    }

    pub fn line(&self) -> usize {
        let locs = self.locations();
        let Some(first_location) = locs.first() else {
            panic!("{:?} is missing location information", self)
        };
        first_location.line
    }
}

#[derive(Serialize, Debug)]
pub struct Section {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    #[serde(rename = "title")]
    inlines: Vec<Inline>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    reftext: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
            inlines: vec![],   // added later
            reftext: vec![], // added later
            metadata: None,
            level,
            blocks: vec![],
            location: vec![first_location],
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
#[serde(rename_all = "lowercase")]
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
#[serde(rename_all = "lowercase")]
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
    #[serde(skip_serializing_if = "LeafBlockForm::is_paragraph")]
    form: LeafBlockForm,
    #[serde(skip_serializing_if = "Option::is_none")]
    delimiter: Option<String>, // if it's a delimited block, then we provide the delimiter
    inlines: Vec<Inline>,
    //blocks: Vec<Block>, // I'm pretty sure there aren't allowed to have blocks, need to confirm
    location: Vec<Location>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LeafBlockName {
    Listing,
    Literal, // TK not handling now
    Paragraph,
    Pass,
    Stem,  // TK not handling now
    Verse, // TK need to figure handling for quotes
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum LeafBlockForm {
    Delimited,
    Indented,
    Paragraph,
}

impl LeafBlockForm {
    fn is_paragraph(&self) -> bool {
        matches!(self, LeafBlockForm::Paragraph)
    }
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
    pub name: ParentBlockName,
    #[serde(skip_serializing_if = "Option::is_none")]
    variant: Option<ParentBlockVarient>,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: String,
    delimiter: String, // TK how to handle NOTE: text...
    blocks: Vec<Block>,
    pub location: Vec<Location>,
}

impl PartialEq for ParentBlock {
    fn eq(&self, other: &Self) -> bool {
        if let Some(variant) = &self.variant {
            if let Some(other_variant) = &other.variant {
                variant == other_variant && self.name == other.name
            } else {
                false
            }
        } else {
            self.name == other.name
        }
    }
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ParentBlockName {
    Admonition,
    Example,
    Sidebar,
    Open,
    Quote,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
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
        delimiter: String,
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

    pub fn new_from_token(token: Token) -> Self {
        match token.token_type() {
            TokenType::SidebarBlock => ParentBlock::new(
                ParentBlockName::Sidebar,
                None,
                token.text(),
                vec![],
                token.locations(),
            ),
            TokenType::ExampleBlock => ParentBlock::new(
                ParentBlockName::Example,
                None,
                token.text(),
                vec![],
                token.locations(),
            ),
            TokenType::QuoteVerseBlock => ParentBlock::new(
                ParentBlockName::Quote,
                None,
                token.text(),
                vec![],
                token.locations(),
            ),
            TokenType::OpenBlock => ParentBlock::new(
                ParentBlockName::Open,
                None,
                token.text(),
                vec![],
                token.locations(),
            ),
            TokenType::NotePara => ParentBlock::new(
                ParentBlockName::Admonition,
                Some(ParentBlockVarient::Note),
                token.text(),
                vec![],
                token.locations(),
            ),
            TokenType::TipPara => ParentBlock::new(
                ParentBlockName::Admonition,
                Some(ParentBlockVarient::Tip),
                token.text(),
                vec![],
                token.locations(),
            ),
            TokenType::ImportantPara => ParentBlock::new(
                ParentBlockName::Admonition,
                Some(ParentBlockVarient::Important),
                token.text(),
                vec![],
                token.locations(),
            ),
            TokenType::CautionPara => ParentBlock::new(
                ParentBlockName::Admonition,
                Some(ParentBlockVarient::Caution),
                token.text(),
                vec![],
                token.locations(),
            ),
            TokenType::WarningPara => ParentBlock::new(
                ParentBlockName::Admonition,
                Some(ParentBlockVarient::Warning),
                token.text(),
                vec![],
                token.locations(),
            ),

            _ => panic!("Tried to create a ParentBlock from an invalid Token."),
        }
    }

    pub fn opening_line(&self) -> usize {
        let Some(first_location) = self.location.first() else {
            panic!(
                "{}",
                format!("Missing location information for: {:?}", self)
            )
        };
        first_location.line
    }
}

#[derive(Serialize, PartialEq, Debug)]
pub struct BlockMetadata {
    attributes: HashMap<String, String>,
    options: Vec<String>,
    roles: Vec<String>,
    location: Vec<Location>,
}
