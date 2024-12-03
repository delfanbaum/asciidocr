use core::panic;
use log::error;
use std::{collections::HashMap, fmt::Display};

use serde::Serialize;

use crate::graph::{
    inlines::Inline,
    lists::{DList, DListItem, List, ListItem, ListVariant},
    macros::target_and_attrs_from_token,
    metadata::ElementMetadata,
    nodes::{Location, NodeTypes},
};
use crate::tokens::{Token, TokenType};

/// Blocks enum, containing any tree blocks
#[derive(Serialize, PartialEq, Clone, Debug)]
#[serde(untagged)]
pub enum Block {
    Section(Section), // sort of a special case but prob needs to be included here
    SectionBody,      // this can probably be deleted
    NonSectionBlockBody(NonSectionBlockBody),
    List(List),
    ListItem(ListItem),
    DList(DList),
    DListItem(DListItem),
    DiscreteHeading, // not handled currently
    Break(Break),
    BlockMacro(BlockMacro),
    LeafBlock(LeafBlock),
    ParentBlock(ParentBlock), // Admonitions are hiding in here
    BlockMetadata(ElementMetadata),
    TableCell(TableCell),
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Block::Section(_) => write!(f, "Section"),
            Block::SectionBody => write!(f, "SectionBody"),
            Block::NonSectionBlockBody(_) => write!(f, "NonSectionBlockBody"),
            Block::List(_) => write!(f, "List"),
            Block::ListItem(_) => write!(f, "ListItem"),
            Block::DList(_) => write!(f, "DList"),
            Block::DListItem(_) => write!(f, "DListItem"),
            Block::DiscreteHeading => write!(f, "DiscreteHeading"),
            Block::Break(_) => write!(f, "Break"),
            Block::BlockMacro(_) => write!(f, "BlockMacro"),
            Block::LeafBlock(_) => write!(f, "LeafBlock"),
            Block::ParentBlock(_) => write!(f, "ParentBlock"),
            Block::BlockMetadata(_) => write!(f, "BlockMetadata"),
            Block::TableCell(_) => write!(f, "TableCell"),
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
            Block::Section(section) => {
                if block.is_section() {
                    if let Some(possible_section) = section.blocks.last_mut() {
                        if possible_section.takes_block_of_type(&block) {
                            possible_section.push_block(block);
                        } else {
                            section.blocks.push(block);
                        }
                    } else {
                        section.blocks.push(block);
                    }
                } else {
                    section.blocks.push(block)
                }
            }
            Block::List(list) => list.add_item(block),
            Block::DList(list) => list.add_item(block),
            Block::ListItem(list_item) => list_item.blocks.push(block),
            Block::DListItem(list_item) => list_item.blocks.push(block),
            Block::ParentBlock(parent_block) => {
                if matches!(block, Block::ListItem(_)) {
                    let Some(last) = parent_block.blocks.last_mut() else {
                        panic!("Attempted to push dangling ListItem to parent block")
                    };
                    if matches!(last, Block::List(_)) {
                        last.push_block(block);
                        last.consolidate_locations();
                    } else {
                        panic!("Attempted to push dangling ListItem to parent block")
                    }
                } else if matches!(parent_block.name, ParentBlockName::Table)
                    && !matches!(block, Block::TableCell(_))
                {
                    // sanity-guard
                    panic!("Attempted to add something other than a TableCell to a Table")
                } else {
                    parent_block.blocks.push(block)
                }
            }
            _ => panic!("push_block not implemented for {}", self),
        }
        self.consolidate_locations();
    }

    pub fn takes_inlines(&self) -> bool {
        matches!(
            self,
            Block::Section(_)
                | Block::LeafBlock(_)
                | Block::ListItem(_)
                | Block::DListItem(_)
                | Block::TableCell(_)
        )
    }

    pub fn push_inline(&mut self, inline: Inline) {
        match self {
            Block::Section(section) => section.inlines.push(inline),
            Block::LeafBlock(block) => block.inlines.push(inline),
            Block::ListItem(list_item) => list_item.add_inline(inline),
            Block::DListItem(list_item) => list_item.add_inline(inline),
            Block::TableCell(table_cell) => table_cell.inlines.push(inline),
            _ => panic!("push_block not implemented for {}", self),
        }
    }

    pub fn takes_block_of_type(&self, block: &Block) -> bool {
        match self {
            Block::Section(check) => {
                if let Some(block_level) = block.level_check() {
                    // higher-level sections can take lower-level sections
                    check.level < block_level
                } else {
                    true
                }
            }
            Block::List(_) => matches!(block, Block::ListItem(_)),
            Block::DList(_) => matches!(block, Block::DListItem(_)),
            Block::ParentBlock(_) | Block::ListItem(_) | Block::DListItem(_) => true,
            _ => false,
        }
    }

    pub fn is_section(&self) -> bool {
        matches!(self, Block::Section(_))
    }

    pub fn is_source_block(&self) -> bool {
        match self {
            Block::LeafBlock(block) => block.name == LeafBlockName::Listing,
            _ => false,
        }
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
            Block::ListItem(list) => list.marker == *"." || list.marker.contains('<'),
            _ => false,
        }
    }

    pub fn is_unordered_list_item(&self) -> bool {
        match self {
            Block::ListItem(list) => list.marker == *"*",
            _ => false,
        }
    }

    pub fn is_definition_list_item(&self) -> bool {
        matches!(self, Block::DListItem(_))
    }

    pub fn is_table(&self) -> bool {
        if let Block::ParentBlock(parent_block) = self {
            parent_block.name == ParentBlockName::Table
        } else {
            false
        }
    }

    pub fn consolidate_table_info(&mut self) {
        let Block::ParentBlock(table) = self else {
            panic!("Incorrect function call: consolidate_table_info on non-table block")
        };
        // check if there is an implicit header
        if table.blocks.len() >= 2 {
            // if the cells in the first row are on the same line, either serves as cols
            // designation
            let first_cell_line = table.blocks[0].line();
            if first_cell_line == table.blocks[1].line() {
                // check for an implicit header
                if first_cell_line == table.location[0].line + 1 {
                    if let Some(ref mut metadata) = table.metadata {
                        metadata.options.push("header".to_string());
                    } else {
                        let mut metadata = ElementMetadata::default();
                        metadata.options.push("header".to_string());
                        table.metadata = Some(metadata)
                    }
                }
                // count for implicit column designation
                let cols = table.blocks.iter().fold(0usize, |acc, block| {
                    acc + (block.line() == first_cell_line) as usize
                });
                if let Some(ref mut metadata) = table.metadata {
                    if !metadata.attributes.contains_key("cols") {
                        metadata
                            .attributes
                            .insert("cols".to_string(), format!("{cols}"));
                    }
                } else {
                    let mut metadata = ElementMetadata::default();
                    metadata
                        .attributes
                        .insert("cols".to_string(), format!("{cols}"));
                    table.metadata = Some(metadata)
                }
            }
            // FOR NOW, make the cols an integer for easier templating.
            let Some(ref mut metadata) = table.metadata else {
                error!("Error creating table at Line: {}", first_cell_line);
                std::process::exit(1)
            };
            metadata.simplify_cols()
        }
    }

    pub fn has_blocks(&self) -> bool {
        match self {
            Block::Section(section) => !section.blocks.is_empty(),
            Block::LeafBlock(_) => false,
            _ => true,
        }
    }

    /// Helper when we need to move child blocks from one Block to another
    pub fn extract_blocks(&mut self) -> Vec<Block> {
        match self {
            Block::List(ref mut list) => list.items.drain(..).collect(),
            _ => {
                let v: Vec<Block> = Vec::new();
                v
            }
        }
    }

    pub fn title(&self) -> Option<Vec<Inline>> {
        match self {
            Block::Section(block) => Some(block.inlines.clone()),
            Block::ParentBlock(block) => Some(block.title.clone()),
            Block::BlockMacro(block) => Some(block.caption.clone()),
            _ => None,
        }
    }

    pub fn inlines(&mut self) -> Vec<&mut Inline> {
        let mut inlines: Vec<&mut Inline> = vec![];
        match self {
            // parents
            Block::Section(block) => {
                inlines.extend(block.inlines.iter_mut());
                for child in block.blocks.iter_mut() {
                    inlines.extend(child.inlines())
                }
            }
            Block::ParentBlock(block) => {
                inlines.extend(block.title.iter_mut());
                for child in block.blocks.iter_mut() {
                    inlines.extend(child.inlines())
                }
            }
            Block::List(block) => {
                for child in block.items.iter_mut() {
                    inlines.extend(child.inlines())
                }
            }
            Block::DList(block) => {
                for child in block.items.iter_mut() {
                    inlines.extend(child.inlines())
                }
            }
            Block::ListItem(block) => {
                inlines.extend(block.principal.iter_mut());
                for child in block.blocks.iter_mut() {
                    inlines.extend(child.inlines())
                }
            }
            Block::DListItem(block) => {
                inlines.extend(block.principal.iter_mut());
                for child in block.blocks.iter_mut() {
                    inlines.extend(child.inlines())
                }
            }
            Block::LeafBlock(block) => inlines.extend(block.inlines.iter_mut()),
            Block::TableCell(block) => inlines.extend(block.inlines.iter_mut()),
            _ => {} // remaining blocks don't have inlines
        }

        inlines
    }

    /// Helper for extracting footnote definitions, replacing the footnote span with a reference
    /// that counts based on what's passed to the function
    pub fn extract_footnote_definitions(
        &mut self,
        footnote_count: usize,
        document_id: &str,
    ) -> Vec<Block> {
        // setup references
        let mut local_count = footnote_count;
        // setup list to return
        let mut extracted: Vec<Block> = Vec::new();

        match self {
            // parents
            Block::Section(block) => {
                for child in block.blocks.iter_mut() {
                    let child_footnoes =
                        child.extract_footnote_definitions(extracted.len(), document_id);
                    local_count += child_footnoes.len();
                    extracted.extend(child_footnoes);
                }
            }
            Block::ParentBlock(block) => {
                for child in block.blocks.iter_mut() {
                    let child_footnoes =
                        child.extract_footnote_definitions(extracted.len(), document_id);
                    local_count += child_footnoes.len();
                    extracted.extend(child_footnoes);
                }
            }
            Block::List(block) => {
                for child in block.items.iter_mut() {
                    let child_footnoes =
                        child.extract_footnote_definitions(extracted.len(), document_id);
                    local_count += child_footnoes.len();
                    extracted.extend(child_footnoes);
                }
            }
            Block::DList(block) => {
                for child in block.items.iter_mut() {
                    let child_footnoes =
                        child.extract_footnote_definitions(extracted.len(), document_id);
                    extracted.extend(child_footnoes);
                }
            }
            Block::ListItem(block) => {
                let child_footnotes = block.extract_footnotes(extracted.len(), document_id);
                extracted.extend(child_footnotes);
            }
            Block::DListItem(block) => {
                let child_footnotes = block.extract_footnotes(extracted.len(), document_id);
                extracted.extend(child_footnotes);
            }
            // nonparents
            Block::LeafBlock(block) => {
                for idx in 0..block.inlines.len() {
                    if block.inlines[idx].is_footnote() {
                        local_count += 1;
                        let inline_span = block.inlines.remove(idx);
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
                        block.inlines.insert(idx, replacement_span);
                    }
                }
            }
            _ => todo!(), // placeholder; probably this is a list of panics that should never be
                          // reached
        }

        extracted
    }

    pub fn create_id(&mut self) {
        if let Block::Section(section) = self {
            if section.id == *"" {
                let mut id = String::new();
                for inline in &section.inlines {
                    id.push_str(&inline.extract_values_to_string())
                }
                id = id.replace(' ', "-");
                section.id = id
            }
        }
    }

    pub fn id_hashes(&self) -> HashMap<String, Vec<Inline>> {
        let mut block_id_hash = HashMap::new();
        if let Some(id) = self.id() {
            if let Some(title) = self.title() {
                block_id_hash.insert(id, title);
            } else {
                block_id_hash.insert(id, vec![]);
            }
        }
        match self {
            Block::Section(block) => {
                for child in block.blocks.iter() {
                    block_id_hash.extend(child.id_hashes());
                }
            }
            Block::ParentBlock(block) => {
                for child in block.blocks.iter() {
                    block_id_hash.extend(child.id_hashes());
                }
            }
            Block::List(block) => {
                for child in block.items.iter() {
                    block_id_hash.extend(child.id_hashes());
                }
            }
            Block::DList(block) => {
                for child in block.items.iter() {
                    block_id_hash.extend(child.id_hashes());
                }
            }
            Block::ListItem(block) => {
                for child in block.blocks.iter() {
                    block_id_hash.extend(child.id_hashes());
                }
            }
            Block::DListItem(block) => {
                for child in block.blocks.iter() {
                    block_id_hash.extend(child.id_hashes());
                }
            }
            _ => {}
        }
        block_id_hash
    }

    pub fn id(&self) -> Option<String> {
        match self {
            Block::Section(block) => {
                if let Some(metadata) = &block.metadata {
                    metadata.attributes.get("id").cloned()
                } else {
                    None
                }
            }
            Block::List(block) => {
                if let Some(metadata) = &block.metadata {
                    metadata.attributes.get("id").cloned()
                } else {
                    None
                }
            }
            Block::ListItem(block) => {
                if let Some(metadata) = &block.metadata {
                    metadata.attributes.get("id").cloned()
                } else {
                    None
                }
            }
            Block::DList(block) => {
                if let Some(metadata) = &block.metadata {
                    metadata.attributes.get("id").cloned()
                } else {
                    None
                }
            }
            Block::DListItem(block) => {
                if let Some(metadata) = &block.metadata {
                    metadata.attributes.get("id").cloned()
                } else {
                    None
                }
            }
            Block::BlockMacro(block) => {
                if let Some(metadata) = &block.metadata {
                    metadata.attributes.get("id").cloned()
                } else {
                    None
                }
            }
            Block::LeafBlock(block) => {
                if let Some(metadata) = &block.metadata {
                    metadata.attributes.get("id").cloned()
                } else {
                    None
                }
            }
            Block::ParentBlock(block) => {
                if let Some(metadata) = &block.metadata {
                    metadata.attributes.get("id").cloned()
                } else {
                    None
                }
            }
            Block::TableCell(block) => {
                if let Some(metadata) = &block.metadata {
                    metadata.attributes.get("id").cloned()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn locations(&self) -> Vec<Location> {
        match self {
            Block::Section(block) => block.location.clone(),
            Block::SectionBody => vec![],
            Block::NonSectionBlockBody(block) => block.location.clone(),
            Block::List(block) => block.location.clone(),
            Block::ListItem(block) => block.location.clone(),
            Block::DList(block) => block.location.clone(),
            Block::DListItem(block) => block.location.clone(),
            Block::DiscreteHeading => vec![],
            Block::Break(block) => block.location.clone(),
            Block::BlockMacro(block) => block.location.clone(),
            Block::LeafBlock(block) => block.location.clone(),
            Block::ParentBlock(block) => block.location.clone(),
            Block::BlockMetadata(block) => block.location.clone(),
            Block::TableCell(block) => block.location.clone(),
        }
    }

    /// adds and reconciles a block location; specifically useful for delimited blocks
    pub fn add_locations(&mut self, locations: Vec<Location>) {
        match self {
            Block::LeafBlock(block) => {
                block.location = Location::reconcile(block.location.clone(), locations)
            }
            _ => todo!(),
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
            Block::DList(list) => {
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
            Block::DListItem(block) => {
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
            Block::TableCell(block) => {
                if let Some(last_inline) = block.inlines.last() {
                    block.location =
                        Location::reconcile(block.location.clone(), last_inline.locations())
                }
            }
            Block::Break(_) => {} // do nothing, since there is nothing to do!
            Block::BlockMacro(_) => {} // do nothing for now; maybe we include the meta
            // locations?
            _ => todo!(),
        }
    }

    pub fn line(&self) -> usize {
        let locs = self.locations();
        let Some(first_location) = locs.first() else {
            panic!("{:?} is missing location information", self)
        };
        first_location.line
    }

    pub fn add_metadata(&mut self, metadata: ElementMetadata) {
        if metadata.is_empty() {
            return;
        }
        // guard against invalid inline use
        if metadata.inline_metadata {
            // TODO this is a warning, not a panic
            panic!("Invalid inline class markup.")
        }
        match self {
            Block::LeafBlock(block) => block.metadata = Some(metadata),
            Block::Section(block) => block.metadata = Some(metadata),
            Block::ParentBlock(block) => block.metadata = Some(metadata),
            Block::BlockMacro(block) => block.metadata = Some(metadata),
            _ => todo!(),
        }
    }

    /// Returns all literal text in a block
    pub fn block_text(&self) -> String {
        let mut block_text = String::new();
        match self {
            Block::Section(block) => {
                for inline in block.title() {
                    block_text.push_str(&inline.extract_values_to_string())
                }

                for block in block.blocks.iter() {
                    block_text.push_str(&block.block_text())
                }
            }
            Block::List(list) => {
                for item in list.items.iter() {
                    block_text.push_str(&item.block_text())
                }
            }
            Block::ListItem(block) => {
                for inline in block.principal.iter() {
                    block_text.push_str(&inline.extract_values_to_string())
                }

                for block in block.blocks.iter() {
                    block_text.push_str(&block.block_text())
                }
            }
            Block::DList(list) => {
                for item in list.items.iter() {
                    block_text.push_str(&item.block_text())
                }
            }
            Block::DListItem(block) => {
                for inline in block.principal.iter() {
                    block_text.push_str(&inline.extract_values_to_string())
                }

                for block in block.blocks.iter() {
                    block_text.push_str(&block.block_text())
                }
            }
            Block::Break(_) => {} // break doesn't have literals
            Block::BlockMacro(block) => {
                for inline in block.caption.iter() {
                    block_text.push_str(&inline.extract_values_to_string())
                }
            }
            Block::LeafBlock(block) => {
                for inline in block.inlines.iter() {
                    block_text.push_str(&inline.extract_values_to_string())
                }
            }
            Block::ParentBlock(block) => {
                for block in block.blocks.iter() {
                    block_text.push_str(&block.block_text())
                }
            }
            Block::BlockMetadata(_) => {}
            Block::TableCell(block) => {
                for inline in block.inlines.iter() {
                    block_text.push_str(&inline.extract_values_to_string())
                }
            }
            _ => todo!(),
        }
        block_text
    }
}

#[derive(Serialize, Clone, Debug)]
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
    metadata: Option<ElementMetadata>,
    pub level: usize,
    pub blocks: Vec<Block>,
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
            inlines: vec![], // added later
            reftext: vec![], // added later
            metadata: None,
            level,
            blocks: vec![],
            location: vec![first_location],
        }
    }

    pub fn title(&self) -> Vec<Inline> {
        self.inlines.clone()
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct NonSectionBlockBody {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<ElementMetadata>,
    blocks: Vec<Block>,
    location: Vec<Location>,
}

// always equal
impl PartialEq for NonSectionBlockBody {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Serialize, Clone, Debug)]
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

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
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

#[derive(Serialize, Clone, Debug)]
pub struct BlockMacro {
    name: BlockMacroName,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: String,
    target: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub caption: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<ElementMetadata>,
    location: Vec<Location>,
}

impl PartialEq for BlockMacro {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Serialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum BlockMacroName {
    Audio,
    Video,
    Image,
    Toc,
}

impl BlockMacro {
    pub fn new(
        name: BlockMacroName,
        target: String,
        metadata: Option<ElementMetadata>,
        location: Vec<Location>,
    ) -> Self {
        BlockMacro {
            name,
            node_type: NodeTypes::Block,
            form: "macro".to_string(),
            target,
            caption: vec![],
            metadata,
            location,
        }
    }

    pub fn new_image_from_token(token: Token) -> Self {
        let (target, metadata) = target_and_attrs_from_token(&token);
        BlockMacro::new(BlockMacroName::Image, target, metadata, token.locations())
    }

    pub fn add_metadata(mut self, incoming_metadata: &ElementMetadata) -> Self {
        match self.metadata {
            Some(ref mut metadata) => metadata.add_metadata_from_other(incoming_metadata),
            None => self.metadata = Some(incoming_metadata.clone()),
        }
        self
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct LeafBlock {
    pub name: LeafBlockName,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    #[serde(skip_serializing_if = "LeafBlockForm::is_paragraph")]
    form: LeafBlockForm,
    #[serde(skip_serializing_if = "Option::is_none")]
    delimiter: Option<String>, // if it's a delimited block, then we provide the delimiter
    inlines: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    location: Vec<Location>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LeafBlockName {
    Listing,
    Literal, // TK not handling now
    Paragraph,
    Pass,
    Stem, // TK not handling now
    Verse,
    Comment, // Gets thrown away, but convenient
}
#[derive(Serialize, Clone, Debug)]
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
            metadata: None,
            location,
        }
    }

    fn new_delimited_block(token: Token, name: LeafBlockName) -> Self {
        LeafBlock {
            name,
            node_type: NodeTypes::Block,
            form: LeafBlockForm::Delimited,
            delimiter: Some(token.text()),
            inlines: vec![],
            metadata: None,
            location: token.locations(),
        }
    }

    /// Creates a delimited block from based on the token type
    pub fn new_from_token(token: Token) -> Self {
        match token.token_type() {
            TokenType::PassthroughBlock => Self::new_delimited_block(token, LeafBlockName::Pass),
            TokenType::LiteralBlock => Self::new_delimited_block(token, LeafBlockName::Literal),
            TokenType::SourceBlock => Self::new_delimited_block(token, LeafBlockName::Listing),
            TokenType::CommentBlock => Self::new_delimited_block(token, LeafBlockName::Listing),
            TokenType::QuoteVerseBlock => Self::new_delimited_block(token, LeafBlockName::Verse),
            _ => panic!(
                "Can't create delimited leaf block from this type of token: {:?}",
                token
            ),
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

    pub fn inlines(&self) -> Vec<Inline> {
        self.inlines.clone()
    }
    pub fn new_footnote_def_from_id_and_inlines(
        definition_id: String,
        inlines: Vec<Inline>,
    ) -> Self {
        Self {
            name: LeafBlockName::Paragraph,
            node_type: NodeTypes::Block,
            form: LeafBlockForm::Paragraph,
            delimiter: None,
            inlines,
            metadata: Some(ElementMetadata::new_with_id_and_roles(
                definition_id,
                vec!["footnote".to_string()],
            )),
            location: vec![],
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct ParentBlock {
    pub name: ParentBlockName,
    #[serde(skip_serializing_if = "Option::is_none")]
    variant: Option<ParentBlockVarient>,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: String, // required as "delimited", but really could also be "paragraph"
    #[serde(skip_serializing_if = "String::is_empty")]
    delimiter: String, // required, but if it should be "paragraph" it's empty
    blocks: Vec<Block>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub title: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
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

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ParentBlockName {
    Admonition,
    Example,
    Sidebar,
    Open,
    Quote,
    Table, // Tables function basically the same in terms of delimiter, so I'm going to reuse
    // ParentBlock until someone convinces me otherwise
    FootnoteContainer,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
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
            title: vec![],
            metadata: None,
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
            TokenType::Table => ParentBlock::new(
                ParentBlockName::Table,
                None,
                token.text(),
                vec![],
                token.locations(),
            ),

            _ => panic!("Tried to create a ParentBlock from an invalid Token."),
        }
    }

    pub fn new_footnotes_container(footnote_defs: Vec<Block>) -> Self {
        Self::new(
            ParentBlockName::FootnoteContainer,
            None,
            "".to_string(),
            footnote_defs,
            vec![],
        )
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

#[derive(Serialize, Clone, Debug)]
pub struct TableCell {
    pub name: String,
    node_type: NodeTypes,
    pub inlines: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    pub location: Vec<Location>,
}

impl PartialEq for TableCell {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl TableCell {
    pub fn new_from_token(token: Token) -> Self {
        TableCell {
            name: "tableCell".to_string(),
            node_type: NodeTypes::Block,
            inlines: vec![],
            metadata: None,
            location: token.locations(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::inlines::{
        InlineLiteral, InlineLiteralName, InlineRefVariant, InlineSpan, InlineSpanForm,
        InlineSpanVariant,
    };

    use super::*;
    use core::panic;

    #[test]
    fn extract_footnote_definitions() {
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
        let mut some_leaf = Block::LeafBlock(LeafBlock::new(
            LeafBlockName::Paragraph,
            LeafBlockForm::Paragraph,
            None,
            vec![],
            vec![footnote],
        ));
        let extracted = some_leaf.extract_footnote_definitions(0, "");
        let Block::LeafBlock(result) = some_leaf else {
            panic!("Destroyed the leaf block somehow")
        };

        let Some(inline) = result.inlines.first() else {
            panic!("Removed the inlines from the block instead of replacing them")
        };
        // ensure we've swapped the span
        if let Inline::InlineSpan(span) = inline {
            assert_eq!(span.variant, InlineSpanVariant::Superscript)
        }
        assert_eq!(result.inlines.len(), 1);

        // ensure our result is what we expect it to be
        assert_eq!(extracted.len(), 1);
        let Some(Block::LeafBlock(block)) = extracted.first() else {
            panic!("Extracted block was not a leaf block")
        };
        assert_eq!(block.name, LeafBlockName::Paragraph);
        let Some(ref metadata) = block.metadata else {
            panic!("Extracted leaf block is missing metadata")
        };
        assert_eq!(metadata.element_id().unwrap(), "_footnotedef_1");
        assert_eq!(metadata.roles.first().unwrap(), "footnote");
        let Some(Inline::InlineRef(inline)) = block.inlines.first() else {
            panic!("Missing footnote def content!")
        };
        assert_eq!(inline.variant, InlineRefVariant::Xref);
        assert_eq!(inline.target, "_footnoteref_1");
    }
}
