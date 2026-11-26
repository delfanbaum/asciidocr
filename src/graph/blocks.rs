use log::warn;
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use serde::Serialize;

use crate::errors::BlockError;
use crate::graph::{
    ids::{IdSettings, generate_id, title_for_id, unique_id},
    inlines::Inline,
    lists::{DList, DListItem, List, ListItem, ListVariant},
    metadata::ElementMetadata,
    nodes::{Location, NodeTypes},
};
use crate::scanner::tokens::{Token, TokenType};

/// Blocks Enum, containing all possible document blocks
#[derive(Serialize, PartialEq, Clone, Debug)]
#[serde(untagged)]
pub enum Block {
    Section(Section),
    /// Current unused, but included in schema
    SectionBody,
    /// Current unused, but included in schema
    NonSectionBlockBody(NonSectionBlockBody),
    List(List),
    ListItem(ListItem),
    DList(DList),
    DListItem(DListItem),
    /// Current unused, but included in schema
    DiscreteHeading,
    Break(Break),
    BlockMacro(BlockMacro),
    LeafBlock(LeafBlock),
    /// Parent blocks also include admonition elements
    ParentBlock(ParentBlock),
    BlockMetadata(ElementMetadata),
    /// Tables aren't explicitly specified in the official schema yet, so this is a temporary
    /// workaround for convenience
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

    pub fn push_block(&mut self, block: Block) -> Result<(), BlockError> {
        match self {
            Block::Section(section) => {
                if block.is_section() {
                    if let Some(possible_section) = section.blocks.last_mut() {
                        // only a (higher-level) section can contain this section; other blocks
                        // that take children (sidebars, tables, etc.) are complete by now
                        if possible_section.is_section()
                            && possible_section.takes_block_of_type(&block)
                        {
                            possible_section.push_block(block)?;
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
                        return Err(BlockError::DanglingList);
                    };
                    if matches!(last, Block::List(_)) {
                        last.push_block(block)?;
                        last.consolidate_locations();
                    } else {
                        return Err(BlockError::DanglingList);
                    }
                } else if matches!(parent_block.name, ParentBlockName::Table)
                    && !matches!(block, Block::TableCell(_))
                {
                    // sanity-guard
                    return Err(BlockError::TableCell);
                } else {
                    parent_block.blocks.push(block)
                }
            }
            _ => return Err(BlockError::NotImplemented(format!("{}", self))),
        }
        self.consolidate_locations();
        Ok(())
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

    pub fn push_inline(&mut self, inline: Inline) -> Result<(), BlockError> {
        match self {
            Block::Section(section) => section.inlines.push(inline),
            Block::LeafBlock(block) => block.inlines.push(inline),
            Block::ListItem(list_item) => list_item.add_inline(inline),
            Block::DListItem(list_item) => list_item.add_inline(inline),
            Block::TableCell(table_cell) => table_cell.inlines.push(inline),
            _ => return Err(BlockError::NotImplemented(format!("{}", self))),
        }
        Ok(())
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

    pub fn consolidate_table_info(&mut self) -> Result<(), BlockError> {
        let Block::ParentBlock(table) = self else {
            return Err(BlockError::IncorrectCall);
        };
        let Some(first_cell) = table.blocks.first() else {
            return Ok(());
        };
        let first_cell_line = first_cell.line()?;
        let cells_on_first_line = table
            .blocks
            .iter()
            .filter(|block| block.line().is_ok_and(|line| line == first_cell_line))
            .count();
        // an implicit header: a first row of several cells, directly under the delimiter
        if cells_on_first_line >= 2 && first_cell_line == table.location[0].line + 1 {
            table
                .metadata
                .get_or_insert_with(ElementMetadata::default)
                .options
                .push("header".to_string());
        }
        // as in Asciidoctor, unless `cols` says otherwise, the cells on the first line are the
        // columns (one, if the first row has a cell per line)
        let metadata = table.metadata.get_or_insert_with(ElementMetadata::default);
        metadata
            .attributes
            .entry("cols".to_string())
            .or_insert_with(|| format!("{cells_on_first_line}"));
        // FOR NOW, make the cols an integer for easier templating.
        metadata.simplify_cols();
        Ok(())
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
            Block::List(list) => list.items.drain(..).collect(),
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
            Block::List(block) => Some(block.title.clone()),
            Block::DList(block) => Some(block.title.clone()),
            _ => None,
        }
    }

    pub fn inlines(&self) -> Vec<Inline> {
        let mut inlines: Vec<Inline> = vec![];
        match self {
            // parents
            Block::Section(block) => {
                inlines.extend(block.inlines.clone());
                for child in block.blocks.iter() {
                    inlines.extend(child.inlines())
                }
            }
            Block::ParentBlock(block) => {
                inlines.extend(block.title.clone());
                for child in block.blocks.iter() {
                    inlines.extend(child.inlines())
                }
            }
            Block::List(block) => {
                inlines.extend(block.title.clone());
                for child in block.items.iter() {
                    inlines.extend(child.inlines())
                }
            }
            Block::DList(block) => {
                inlines.extend(block.title.clone());
                for child in block.items.iter() {
                    inlines.extend(child.inlines())
                }
            }
            Block::ListItem(block) => {
                inlines.extend(block.principal.clone());
                for child in block.blocks.iter() {
                    inlines.extend(child.inlines())
                }
            }
            Block::DListItem(block) => {
                inlines.extend(block.principal.clone());
                for child in block.blocks.iter() {
                    inlines.extend(child.inlines())
                }
            }
            Block::LeafBlock(block) => {
                inlines.extend(block.title.clone());
                inlines.extend(block.inlines.clone())
            }
            Block::TableCell(block) => inlines.extend(block.inlines.clone()),
            _ => {} // remaining blocks don't have inlines
        }

        inlines
    }
    pub fn inlines_mut(&mut self) -> Vec<&mut Inline> {
        let mut inlines: Vec<&mut Inline> = vec![];
        match self {
            // parents
            Block::Section(block) => {
                inlines.extend(block.inlines.iter_mut());
                for child in block.blocks.iter_mut() {
                    inlines.extend(child.inlines_mut())
                }
            }
            Block::ParentBlock(block) => {
                inlines.extend(block.title.iter_mut());
                for child in block.blocks.iter_mut() {
                    inlines.extend(child.inlines_mut())
                }
            }
            Block::List(block) => {
                inlines.extend(block.title.iter_mut());
                for child in block.items.iter_mut() {
                    inlines.extend(child.inlines_mut())
                }
            }
            Block::DList(block) => {
                inlines.extend(block.title.iter_mut());
                for child in block.items.iter_mut() {
                    inlines.extend(child.inlines_mut())
                }
            }
            Block::ListItem(block) => {
                inlines.extend(block.principal.iter_mut());
                for child in block.blocks.iter_mut() {
                    inlines.extend(child.inlines_mut())
                }
            }
            Block::DListItem(block) => {
                inlines.extend(block.principal.iter_mut());
                for child in block.blocks.iter_mut() {
                    inlines.extend(child.inlines_mut())
                }
            }
            Block::LeafBlock(block) => {
                inlines.extend(block.title.iter_mut());
                inlines.extend(block.inlines.iter_mut())
            }
            Block::TableCell(block) => inlines.extend(block.inlines.iter_mut()),
            _ => {} // remaining blocks don't have inlines
        }

        inlines
    }

    pub fn create_id(&mut self) {
        if let Block::Section(section) = self
            && section.id == *""
        {
            let mut id = String::new();
            for inline in &section.inlines {
                id.push_str(&inline.extract_values_to_string())
            }
            id = id.replace(' ', "-");
            section.id = id
        }
    }

    /// The blocks directly inside this one (a section's, a list's items, ...)
    pub fn child_blocks(&self) -> &[Block] {
        match self {
            Block::Section(block) => &block.blocks,
            Block::ParentBlock(block) => &block.blocks,
            Block::List(block) => &block.items,
            Block::DList(block) => &block.items,
            Block::ListItem(block) => &block.blocks,
            Block::DListItem(block) => &block.blocks,
            _ => &[],
        }
    }

    /// A section's ID: the one it was given, or the one generated for it
    pub fn section_id(&self) -> Option<String> {
        match self {
            Block::Section(section) => self.id().or_else(|| section.generated_id.clone()),
            _ => None,
        }
    }

    /// Generates IDs for the sections in this block (itself included) that weren't given one, in
    /// document order, unique among the IDs in `taken` (which they are added to, as are the
    /// explicit IDs met on the way); each generated ID and its section's title go in `titles`, for
    /// cross references
    pub fn generate_section_ids(
        &mut self,
        settings: &IdSettings,
        taken: &mut HashSet<String>,
        titles: &mut HashMap<String, Vec<Inline>>,
    ) {
        if let Some(id) = self.id() {
            taken.insert(id);
        } else if let Block::Section(section) = self
            && settings.enabled
        {
            let id = unique_id(
                generate_id(&title_for_id(&section.inlines), settings),
                settings,
                taken,
            );
            taken.insert(id.clone());
            titles.insert(id.clone(), section.inlines.clone());
            section.generated_id = Some(id);
        }
        let children: &mut Vec<Block> = match self {
            Block::Section(block) => &mut block.blocks,
            Block::ParentBlock(block) => &mut block.blocks,
            Block::List(block) => &mut block.items,
            Block::DList(block) => &mut block.items,
            Block::ListItem(block) => &mut block.blocks,
            Block::DListItem(block) => &mut block.blocks,
            _ => return,
        };
        for child in children.iter_mut() {
            child.generate_section_ids(settings, taken, titles);
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
            _ => warn!("Unresolved locations at {}", self),
        }
    }

    pub fn consolidate_locations(&mut self) {
        match self {
            Block::Section(block) => {
                // consolidate the inlines/title
                if let Some(last_inline) = block.inlines.last() {
                    block.location =
                        Location::reconcile(block.location.clone(), last_inline.locations())
                }
                if let Some(last_block) = block.blocks.last() {
                    block.location =
                        Location::reconcile(block.location.clone(), last_block.locations())
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
            // never created (and nothing to consolidate if they were)
            Block::SectionBody
            | Block::NonSectionBlockBody(_)
            | Block::DiscreteHeading
            | Block::BlockMetadata(_) => {}
        }
    }

    pub fn line(&self) -> Result<usize, BlockError> {
        let locs = self.locations();
        match locs.first() {
            Some(first_location) => Ok(first_location.line),
            None => Err(BlockError::Location),
        }
    }

    pub fn add_metadata(&mut self, metadata: ElementMetadata) -> Result<(), BlockError> {
        if metadata.is_empty() {
            return Ok(());
        }
        // guard against invalid inline use
        if metadata.inline_metadata {
            // TODO this is a warning, not a panic
            warn!("Invalid inline class markup.")
        }
        match self {
            Block::Section(block) => block.metadata = Some(metadata),
            Block::List(block) => block.metadata = Some(metadata),
            Block::ListItem(block) => block.metadata = Some(metadata),
            Block::DList(block) => block.metadata = Some(metadata),
            Block::DListItem(block) => block.metadata = Some(metadata),
            Block::BlockMacro(block) => block.metadata = Some(metadata),
            Block::LeafBlock(block) => block.metadata = Some(metadata),
            Block::ParentBlock(block) => block.metadata = Some(metadata),
            Block::TableCell(block) => block.metadata = Some(metadata),
            _ => {
                return Err(BlockError::InvalidMetadata(format!("{}", self)));
            }
        }
        Ok(())
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
            // never created (and have no text if they were)
            Block::SectionBody | Block::NonSectionBlockBody(_) | Block::DiscreteHeading => {}
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
    /// the ID generated for a section that wasn't given one (see [`crate::graph::ids`]); not part
    /// of the JSON (TCK) output
    #[serde(skip)]
    pub generated_id: Option<String>,
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
            generated_id: None,
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
    pub variant: BreakVariant,
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
    pub name: BlockMacroName,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: String,
    pub target: String,
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

    pub fn new_image_block(
        target: String,
        metadata: Option<ElementMetadata>,
        locations: Vec<Location>,
    ) -> Self {
        BlockMacro::new(BlockMacroName::Image, target, metadata, locations)
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub title: Vec<Inline>,
    inlines: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    location: Vec<Location>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LeafBlockName {
    Listing,
    Literal,
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
            title: vec![],
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
            title: vec![],
            inlines: vec![],
            metadata: None,
            location: token.locations(),
        }
    }

    /// Creates a delimited block from based on the token type
    pub fn new_from_token(token: Token) -> Result<Self, BlockError> {
        match token.token_type() {
            TokenType::PassthroughBlock => {
                Ok(Self::new_delimited_block(token, LeafBlockName::Pass))
            }
            TokenType::LiteralBlock => Ok(Self::new_delimited_block(token, LeafBlockName::Literal)),
            TokenType::SourceBlock => Ok(Self::new_delimited_block(token, LeafBlockName::Listing)),
            TokenType::CommentBlock => Ok(Self::new_delimited_block(token, LeafBlockName::Listing)),
            TokenType::QuoteVerseBlock => {
                Ok(Self::new_delimited_block(token, LeafBlockName::Verse))
            }
            _ => Err(BlockError::InvalidToken),
        }
    }

    pub fn opening_line(&self) -> Result<usize, BlockError> {
        match self.location.first() {
            Some(first_location) => Ok(first_location.line),
            None => Err(BlockError::Location),
        }
    }

    pub fn inlines(&self) -> Vec<Inline> {
        self.inlines.clone()
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct ParentBlock {
    pub name: ParentBlockName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<ParentBlockVarient>,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: String, // required as "delimited", but really could also be "paragraph"
    #[serde(skip_serializing_if = "String::is_empty")]
    delimiter: String, // required, but if it should be "paragraph" it's empty
    pub blocks: Vec<Block>,
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

impl Display for ParentBlockVarient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParentBlockVarient::Caution => write!(f, "Caution"),
            ParentBlockVarient::Important => write!(f, "Important"),
            ParentBlockVarient::Note => write!(f, "Note"),
            ParentBlockVarient::Tip => write!(f, "Tip"),
            ParentBlockVarient::Warning => write!(f, "Warning"),
        }
    }
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

    pub fn new_from_token(token: Token) -> Result<Self, BlockError> {
        match token.token_type() {
            TokenType::SidebarBlock => Ok(ParentBlock::new(
                ParentBlockName::Sidebar,
                None,
                token.text(),
                vec![],
                token.locations(),
            )),
            TokenType::ExampleBlock => Ok(ParentBlock::new(
                ParentBlockName::Example,
                None,
                token.text(),
                vec![],
                token.locations(),
            )),
            TokenType::QuoteVerseBlock => Ok(ParentBlock::new(
                ParentBlockName::Quote,
                None,
                token.text(),
                vec![],
                token.locations(),
            )),
            TokenType::OpenBlock => Ok(ParentBlock::new(
                ParentBlockName::Open,
                None,
                token.text(),
                vec![],
                token.locations(),
            )),
            TokenType::NotePara => Ok(ParentBlock::new(
                ParentBlockName::Admonition,
                Some(ParentBlockVarient::Note),
                token.text(),
                vec![],
                token.locations(),
            )),
            TokenType::TipPara => Ok(ParentBlock::new(
                ParentBlockName::Admonition,
                Some(ParentBlockVarient::Tip),
                token.text(),
                vec![],
                token.locations(),
            )),
            TokenType::ImportantPara => Ok(ParentBlock::new(
                ParentBlockName::Admonition,
                Some(ParentBlockVarient::Important),
                token.text(),
                vec![],
                token.locations(),
            )),
            TokenType::CautionPara => Ok(ParentBlock::new(
                ParentBlockName::Admonition,
                Some(ParentBlockVarient::Caution),
                token.text(),
                vec![],
                token.locations(),
            )),
            TokenType::WarningPara => Ok(ParentBlock::new(
                ParentBlockName::Admonition,
                Some(ParentBlockVarient::Warning),
                token.text(),
                vec![],
                token.locations(),
            )),
            TokenType::Table => Ok(ParentBlock::new(
                ParentBlockName::Table,
                None,
                token.text(),
                vec![],
                token.locations(),
            )),

            _ => Err(BlockError::InvalidToken),
        }
    }

    pub fn opening_line(&self) -> Result<usize, BlockError> {
        match self.location.first() {
            Some(first_location) => Ok(first_location.line),
            None => Err(BlockError::Location),
        }
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
