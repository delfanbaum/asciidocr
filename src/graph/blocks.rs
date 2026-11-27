//! Blocks: sections, paragraphs, delimited blocks, tables and so on

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
    lists::{DList, DListItem, List, ListItem},
    metadata::ElementMetadata,
    nodes::{Location, NodeTypes},
};
use crate::scanner::{Token, TokenType};

/// Blocks Enum, containing all possible document blocks
#[derive(Serialize, PartialEq, Clone, Debug)]
#[serde(untagged)]
#[non_exhaustive]
pub enum Block {
    /// A section: a heading and what is under it
    Section(Section),
    /// An ordered or unordered list
    List(List),
    /// An item of a [`Block::List`]
    ListItem(ListItem),
    /// A description list (`term:: description`)
    DList(DList),
    /// An item of a [`Block::DList`]
    DListItem(DListItem),
    /// A page break or thematic break
    Break(Break),
    /// A block macro, such as a block image
    BlockMacro(BlockMacro),
    /// A block that holds text: a paragraph, listing, verse, etc.
    LeafBlock(LeafBlock),
    /// Parent blocks also include admonition elements
    ParentBlock(ParentBlock),
    /// Tables aren't explicitly specified in the official schema yet, so this is a temporary
    /// workaround for convenience
    TableCell(TableCell),
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Block::Section(_) => write!(f, "Section"),
            Block::List(_) => write!(f, "List"),
            Block::ListItem(_) => write!(f, "ListItem"),
            Block::DList(_) => write!(f, "DList"),
            Block::DListItem(_) => write!(f, "DListItem"),
            Block::Break(_) => write!(f, "Break"),
            Block::BlockMacro(_) => write!(f, "BlockMacro"),
            Block::LeafBlock(_) => write!(f, "LeafBlock"),
            Block::ParentBlock(_) => write!(f, "ParentBlock"),
            Block::TableCell(_) => write!(f, "TableCell"),
        }
    }
}

impl Block {
    pub(crate) fn push_block(&mut self, block: Block) -> Result<(), BlockError> {
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

    pub(crate) fn takes_inlines(&self) -> bool {
        matches!(
            self,
            Block::Section(_)
                | Block::LeafBlock(_)
                | Block::ListItem(_)
                | Block::DListItem(_)
                | Block::TableCell(_)
        )
    }

    pub(crate) fn push_inline(&mut self, inline: Inline) -> Result<(), BlockError> {
        match self {
            Block::Section(section) => section.inlines.push(inline),
            Block::LeafBlock(block) => block.inlines.push(inline),
            Block::ListItem(list_item) => list_item.add_inline(inline),
            Block::DListItem(list_item) => list_item.add_inline(inline),
            Block::TableCell(table_cell) => table_cell.push_inline(inline),
            _ => return Err(BlockError::NotImplemented(format!("{}", self))),
        }
        Ok(())
    }

    pub(crate) fn takes_block_of_type(&self, block: &Block) -> bool {
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

    /// Whether this is a section
    pub fn is_section(&self) -> bool {
        matches!(self, Block::Section(_))
    }

    pub(crate) fn is_source_block(&self) -> bool {
        match self {
            Block::LeafBlock(block) => block.name == LeafBlockName::Listing,
            _ => false,
        }
    }

    pub(crate) fn level_check(&self) -> Option<usize> {
        match self {
            Block::Section(section) => Some(section.level),
            _ => None,
        }
    }

    /// Whether this is a list item (of an ordered or unordered list)
    pub fn is_list_item(&self) -> bool {
        matches!(self, Block::ListItem(_))
    }

    /// Whether this is a table
    pub fn is_table(&self) -> bool {
        if let Block::ParentBlock(parent_block) = self {
            parent_block.name == ParentBlockName::Table
        } else {
            false
        }
    }

    pub(crate) fn consolidate_table_info(&mut self) -> Result<(), BlockError> {
        let Block::ParentBlock(table) = self else {
            return Err(BlockError::IncorrectCall);
        };
        let Some(first_cell) = table.blocks.first() else {
            return Ok(());
        };
        let first_cell_start = first_cell.locations().first().cloned().unwrap_or_default();
        let first_cell_line = first_cell_start.line;
        // the first row is the cells on the first line, and on the lines a cell there runs onto
        let mut first_row_end = first_cell_line;
        let mut cells_in_first_row = 0;
        for cell in &table.blocks {
            let start = cell.locations().first().cloned().unwrap_or_default();
            // (an included file counts its own lines)
            if start.file != first_cell_start.file
                || !(first_cell_line..=first_row_end).contains(&start.line)
            {
                break;
            }
            cells_in_first_row += 1;
            first_row_end = cell
                .inlines()
                .iter()
                .flat_map(|inline| inline.locations())
                .map(|location| location.line)
                .fold(first_row_end, usize::max);
        }
        let next_line = match table.blocks.get(cells_in_first_row) {
            Some(cell) => Some(cell.line()?),
            None => table.location.last().map(|end| end.line),
        };
        let asks_for = |wanted: &str| {
            table.metadata.as_ref().is_some_and(|metadata| {
                metadata
                    .attributes
                    .iter()
                    .any(|(name, value)| match name.as_str() {
                        "options" | "opts" => {
                            value.split([',', ' ']).any(|option| option == wanted)
                        }
                        "positional_1" => value.split('%').skip(1).any(|option| option == wanted),
                        _ => false,
                    })
            })
        };
        // an implicit header: a first row of several cells on one line, directly under the
        // delimiter, with a blank line after it
        let implicit_header = cells_in_first_row >= 2
            && first_cell_line == table.location[0].line + 1
            && first_row_end == first_cell_line
            && next_line.is_some_and(|line| line != first_cell_line + 1)
            && !asks_for("noheader");
        // or one asked for: `[%header]` or `[options="header"]`
        let explicit_header = asks_for("header");
        if implicit_header || explicit_header {
            table
                .metadata
                .get_or_insert_with(ElementMetadata::default)
                .options
                .push("header".to_string());
        }
        // as in Asciidoctor, unless `cols` says otherwise, the first row's cells are the
        // columns (one, if there is a cell per line)
        let metadata = table.metadata.get_or_insert_with(ElementMetadata::default);
        metadata
            .attributes
            .entry("cols".to_string())
            .or_insert_with(|| format!("{cells_in_first_row}"));
        // FOR NOW, make the cols an integer for easier templating.
        metadata.simplify_cols();
        Ok(())
    }

    /// The block's attributes, roles and options, if it was given any
    pub fn metadata(&self) -> Option<&ElementMetadata> {
        match self {
            Block::Section(block) => block.metadata.as_ref(),
            Block::List(block) => block.metadata.as_ref(),
            Block::ListItem(block) => block.metadata.as_ref(),
            Block::DList(block) => block.metadata.as_ref(),
            Block::DListItem(block) => block.metadata.as_ref(),
            Block::BlockMacro(block) => block.metadata.as_ref(),
            Block::LeafBlock(block) => block.metadata.as_ref(),
            Block::ParentBlock(block) => block.metadata.as_ref(),
            Block::TableCell(block) => block.metadata.as_ref(),
            Block::Break(_) => None,
        }
    }

    /// The title of a section, a delimited block that holds blocks, a list or description list, or
    /// a block macro (its caption); `None` for other blocks. A [`LeafBlock`]'s title is its
    /// `title` field
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

    /// All the inlines of this block, followed down through the blocks inside it, in document
    /// order (including most blocks' titles)
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
    pub(crate) fn inlines_mut(&mut self) -> Vec<&mut Inline> {
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
    pub(crate) fn generate_section_ids(
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

    pub(crate) fn id_hashes(&self) -> HashMap<String, Vec<Inline>> {
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

    /// The ID the block was given (`[[id]]` or `id=`), if any; see also [`Block::section_id`]
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

    /// Where the block starts and ends in the source
    pub fn locations(&self) -> Vec<Location> {
        match self {
            Block::Section(block) => block.location.clone(),
            Block::List(block) => block.location.clone(),
            Block::ListItem(block) => block.location.clone(),
            Block::DList(block) => block.location.clone(),
            Block::DListItem(block) => block.location.clone(),
            Block::Break(block) => block.location.clone(),
            Block::BlockMacro(block) => block.location.clone(),
            Block::LeafBlock(block) => block.location.clone(),
            Block::ParentBlock(block) => block.location.clone(),
            Block::TableCell(block) => block.location.clone(),
        }
    }

    /// adds and reconciles a block location; specifically useful for delimited blocks
    pub(crate) fn add_locations(&mut self, locations: Vec<Location>) {
        match self {
            Block::LeafBlock(block) => {
                block.location = Location::reconcile(block.location.clone(), locations)
            }
            _ => warn!("Unresolved locations at {}", self),
        }
    }

    pub(crate) fn consolidate_locations(&mut self) {
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
        }
    }

    /// The line the block starts on
    pub fn line(&self) -> Result<usize, BlockError> {
        let locs = self.locations();
        match locs.first() {
            Some(first_location) => Ok(first_location.line),
            None => Err(BlockError::Location),
        }
    }

    pub(crate) fn add_metadata(&mut self, metadata: ElementMetadata) -> Result<(), BlockError> {
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
            Block::TableCell(block) => {
                for inline in block.inlines.iter() {
                    block_text.push_str(&inline.extract_values_to_string())
                }
            }
        }
        block_text
    }

    #[allow(dead_code)] // unused for now; see the cleanup pass in the roadmap
    pub(crate) fn last_inline(&mut self) -> Option<&mut Inline> {
        match self {
            Block::LeafBlock(block) => Some(block.inlines.last_mut()?),
            _ => None,
        }
    }
}

/// A heading and the blocks under it
#[derive(Serialize, Clone, Debug)]
pub struct Section {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    /// The section's ID in the JSON output; use [`Block::section_id`] for the ID the section has
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    #[serde(rename = "title")]
    inlines: Vec<Inline>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    reftext: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<ElementMetadata>,
    /// The heading level: 1 for `==`, up to 5 for `======`, and 0 for a `= Title` that isn't the
    /// document's
    pub level: usize,
    /// The blocks in the section, subsections included
    pub blocks: Vec<Block>,
    location: Vec<Location>,
    /// the ID generated for a section that wasn't given one (see [`crate::graph::ids`]); not part
    /// of the JSON (TCK) output
    #[serde(skip)]
    pub(crate) generated_id: Option<String>,
}

impl PartialEq for Section {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Section {
    // generated as a result of a heading
    pub(crate) fn new(id: String, level: usize, first_location: Location) -> Self {
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

    /// The heading's text
    pub fn title(&self) -> Vec<Inline> {
        self.inlines.clone()
    }
}

/// A page break (`<<<`) or thematic break (`'''`)
#[derive(Serialize, Clone, Debug)]
pub struct Break {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    /// Which kind of break
    pub variant: BreakVariant,
    location: Vec<Location>,
}

impl PartialEq for Break {
    fn eq(&self, other: &Self) -> bool {
        self.variant == other.variant
    }
}

/// The kinds of break
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum BreakVariant {
    /// A page break
    Page,
    /// A thematic break (a horizontal rule)
    Thematic,
}

impl Break {
    pub(crate) fn new(variant: BreakVariant, location: Vec<Location>) -> Self {
        Break {
            name: "break".to_string(),
            node_type: NodeTypes::Block,
            variant,
            location,
        }
    }
}

/// A macro on a line of its own, such as `image::target[]`
#[derive(Serialize, Clone, Debug)]
pub struct BlockMacro {
    /// Which macro
    pub name: BlockMacroName,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: String,
    /// What the macro points at (an image's path or URL)
    pub target: String,
    /// The caption, from a block title above the macro
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

/// The block macros
#[derive(Serialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum BlockMacroName {
    /// An audio clip (not parsed yet)
    Audio,
    /// A video (not parsed yet)
    Video,
    /// An image
    Image,
    /// A table of contents (not parsed yet)
    Toc,
}

impl BlockMacro {
    pub(crate) fn new(
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

    pub(crate) fn new_image_block(
        target: String,
        metadata: Option<ElementMetadata>,
        locations: Vec<Location>,
    ) -> Self {
        BlockMacro::new(BlockMacroName::Image, target, metadata, locations)
    }

    pub(crate) fn add_metadata(mut self, incoming_metadata: &ElementMetadata) -> Self {
        match self.metadata {
            Some(ref mut metadata) => metadata.add_metadata_from_other(incoming_metadata),
            None => self.metadata = Some(incoming_metadata.clone()),
        }
        self
    }
}

/// A block that holds text rather than other blocks
#[derive(Serialize, Clone, Debug)]
pub struct LeafBlock {
    /// Which kind of block
    pub name: LeafBlockName,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    #[serde(skip_serializing_if = "LeafBlockForm::is_paragraph")]
    form: LeafBlockForm,
    #[serde(skip_serializing_if = "Option::is_none")]
    delimiter: Option<String>, // if it's a delimited block, then we provide the delimiter
    /// The block title above it, if any
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub title: Vec<Inline>,
    inlines: Vec<Inline>,
    /// The block's attributes, roles and options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    location: Vec<Location>,
}

/// The kinds of block that hold text
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum LeafBlockName {
    /// A source or listing block (`----`)
    Listing,
    /// A literal block (`....`)
    Literal,
    /// A paragraph
    Paragraph,
    /// A passthrough block (`++++`), whose content is output as written
    Pass,
    /// Equations (not parsed yet)
    Stem, // TK not handling now
    /// A verse (`[verse]` above `____`)
    Verse,
    /// A comment block; these aren't kept in the graph
    Comment, // Gets thrown away, but convenient
}
/// How a block was written
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum LeafBlockForm {
    /// Between delimiter lines
    Delimited,
    /// As indented lines (not parsed yet)
    Indented,
    /// As a plain paragraph
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
    pub(crate) fn new(
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
    pub(crate) fn new_from_token(token: Token) -> Result<Self, BlockError> {
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

    /// The block's text
    pub fn inlines(&self) -> Vec<Inline> {
        self.inlines.clone()
    }

    #[allow(dead_code)] // unused for now; see the cleanup pass in the roadmap
    pub(crate) fn opening_line(&self) -> Result<usize, BlockError> {
        match self.location.first() {
            Some(first_location) => Ok(first_location.line),
            None => Err(BlockError::Location),
        }
    }
}

/// A block that holds other blocks
#[derive(Serialize, Clone, Debug)]
pub struct ParentBlock {
    /// Which kind of block
    pub name: ParentBlockName,
    /// An admonition's label (note, tip, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<ParentBlockVarient>,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    form: String, // required as "delimited", but really could also be "paragraph"
    #[serde(skip_serializing_if = "String::is_empty")]
    delimiter: String, // required, but if it should be "paragraph" it's empty
    /// The blocks inside; for a table, its cells, row after row
    pub blocks: Vec<Block>,
    /// The block title above it, if any
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub title: Vec<Inline>,
    /// The block's attributes, roles and options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    /// Where the block starts and ends in the source
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

/// The kinds of block that hold blocks
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ParentBlockName {
    /// An admonition (`NOTE: text`, or `[NOTE]` above a block)
    Admonition,
    /// An example (`====`)
    Example,
    /// A sidebar (`****`)
    Sidebar,
    /// An open block (`--`)
    Open,
    /// A quote (`____`)
    Quote,
    /// A table (`|===`)
    Table, // Tables function basically the same in terms of delimiter, so I'm going to reuse
           // ParentBlock until someone convinces me otherwise
}

/// The labels of admonitions
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ParentBlockVarient {
    /// `CAUTION`
    Caution,
    /// `IMPORTANT`
    Important,
    /// `NOTE`
    Note,
    /// `TIP`
    Tip,
    /// `WARNING`
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
    pub(crate) fn new(
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

    pub(crate) fn new_from_token(token: Token) -> Result<Self, BlockError> {
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

    pub(crate) fn opening_line(&self) -> Result<usize, BlockError> {
        match self.location.first() {
            Some(first_location) => Ok(first_location.line),
            None => Err(BlockError::Location),
        }
    }
}

/// A table cell
#[derive(Serialize, Clone, Debug)]
pub struct TableCell {
    /// Always `tableCell`
    pub name: String,
    node_type: NodeTypes,
    /// The cell's text
    pub inlines: Vec<Inline>,
    /// The cell's attributes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    /// Where the cell is in the source
    pub location: Vec<Location>,
}

impl PartialEq for TableCell {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl TableCell {
    /// Adds an inline, joining text to the text before it (a cell's later lines, say)
    fn push_inline(&mut self, inline: Inline) {
        if let (Some(Inline::InlineLiteral(last)), Inline::InlineLiteral(literal)) =
            (self.inlines.last_mut(), &inline)
            && last.can_merge_with(literal)
            && literal.value().starts_with('\n')
        {
            last.combine_literals(literal.clone());
        } else {
            self.inlines.push(inline)
        }
    }

    pub(crate) fn new_from_token(token: Token) -> Self {
        TableCell {
            name: "tableCell".to_string(),
            node_type: NodeTypes::Block,
            inlines: vec![],
            metadata: None,
            location: token.locations(),
        }
    }
}
