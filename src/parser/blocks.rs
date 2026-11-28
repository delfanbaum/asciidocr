//! Blocks: sections, delimited blocks, tables, admonitions, breaks, block images and the
//! attribute lines above blocks

use std::collections::VecDeque;

use log::warn;

use crate::scanner::{Token, TokenType};
use crate::{errors::ParserError, scanner::Scanner};
use crate::{
    graph::metadata::target_and_attrs_from_token,
    graph::{
        asg::Asg,
        blocks::{Block, BlockMacro, Break, LeafBlock, ParentBlock, Section, TableCell},
        header::Header,
        location::Location,
    },
};

use super::{HeaderLine, Parser};

/// The deepest section level, an `h6` in HTML
const MAX_SECTION_LEVEL: i32 = 5;

impl Parser {
    pub(super) fn parse_block_anchor_attributes(
        &mut self,
        token: Token,
    ) -> Result<(), ParserError> {
        self.add_metadata_from_token(token);
        self.force_new_block = true;
        Ok(())
    }

    /// parses element attribute lists into self.block_metadata, which then is applied later
    pub(super) fn parse_block_element_attributes(
        &mut self,
        token: Token,
    ) -> Result<(), ParserError> {
        if ["[stem]", "[latexmath]", "[asciimath]"].contains(&token.lexeme.as_str()) {
            warn!(
                "Line {}: Equations ({}) are not supported",
                token.line, token.lexeme
            );
        }
        self.add_metadata_from_token(token);
        self.force_new_block = true;
        Ok(())
    }

    pub(super) fn parse_thematic_break(
        &mut self,
        token: Token,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        self.add_to_block_stack_or_graph(
            asg,
            Block::Break(Break::new(
                crate::graph::blocks::BreakVariant::Thematic,
                token.locations(),
            )),
        )
    }

    pub(super) fn parse_page_break(
        &mut self,
        token: Token,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        self.add_to_block_stack_or_graph(
            asg,
            Block::Break(Break::new(
                crate::graph::blocks::BreakVariant::Page,
                token.locations(),
            )),
        )
    }

    // Comments
    pub(super) fn parse_comment(&self) -> Result<(), ParserError> {
        // for now, do nothing
        Ok(())
    }

    //fn parse_block_label(&mut self, token: Token, asg: &mut Asg) {}

    pub(super) fn parse_title(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        // Check offsets
        if self.level_offset != 0 {
            return self.parse_section_headings(token, asg);
        }

        // the title can have comments, attribute entries and blank lines above it, but no content
        if self.document_header.is_none()
            && asg.header.is_none()
            && asg.blocks.is_empty()
            && self.block_stack.is_empty()
            && self.lists.is_empty()
            && self.inline_stack.is_empty()
        {
            self.in_block_line = true;
            // the newlines above the title aren't part of it
            self.dangling_newline = None;
            self.header_line = HeaderLine::Title;
            self.header_line_text.clear();
            // (they would otherwise end up on the first block)
            if self.metadata.take().is_some() {
                warn!(
                    "Line {}: Attributes above the document title are not supported; ignoring them",
                    token.line
                );
            }
            let mut header = Header::new();
            header.location.extend(token.locations());

            self.document_header = Some(header);
            Ok(())
        } else {
            self.parse_section_headings(token, asg)
        }
    }

    pub(super) fn parse_section_headings(
        &mut self,
        token: Token,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        let level = self.get_heading_level(&token)?;
        // like Asciidoctor, report it and carry on (only a book may have these, as parts)
        if level == 0
            && self
                .document_attributes
                .get("doctype")
                .is_none_or(|doctype| doctype != "book")
        {
            warn!(
                "Line {}: Level 0 headings are only allowed at the top of a document",
                token.line
            );
        }

        // if the last section is at the same level, we need to push that up, otherwise the
        // accordion effect gets screwy with section levels
        if let Some(Block::Section(_)) = self.block_stack.last() {
            self.add_last_to_block_stack_or_graph(asg)?
        }
        //if let Some(Block::Section(last_section)) = self.block_stack.last() {
        //    if last_section.level >= level {
        //        self.add_last_to_block_stack_or_graph(asg)
        //    }
        //}
        // add the new section to the stack
        self.push_block_to_stack(Block::Section(Section::new(
            "".to_string(),
            level,
            token.first_location(),
        )))?;
        // let us know we're in a block line
        self.in_block_line = true;
        self.section_title_open = true;
        // clear any dangling newlines, since we don't want these added to the title
        self.dangling_newline = None;
        // let us know that we want to add to the section title for a little bit
        self.force_new_block = false;
        Ok(())
    }

    /// Returns the level of a heading, with the level offset applied. Like Asciidoctor, an
    /// offset never stops the conversion; a level it puts out of range is brought into it
    fn get_heading_level(&self, token: &Token) -> Result<usize, ParserError> {
        let level: i32 = match token.token_type() {
            TokenType::Heading1 => 0,
            TokenType::Heading2 => 1,
            TokenType::Heading3 => 2,
            TokenType::Heading4 => 3,
            TokenType::Heading5 => 4,
            TokenType::Heading6 => 5,
            _ => {
                return Err(ParserError::InternalError(
                    self.line,
                    "Inavlid token given to parse_section_headings".to_string(),
                ));
            }
        };
        let level = level.saturating_add(self.level_offset);
        if level > MAX_SECTION_LEVEL {
            warn!(
                "Line {}: Heading level {level} is out of range; using {MAX_SECTION_LEVEL}",
                token.line
            );
        }
        // (a level below 0 is reported as the level 0 it becomes)
        Ok(level.clamp(0, MAX_SECTION_LEVEL) as usize)
    }

    pub(super) fn parse_admonition_para_syntax(&mut self, token: Token) -> Result<(), ParserError> {
        let mut admonition = ParentBlock::new_from_token(token)?;
        // a block title above an inline-style admonition (`NOTE: text`) belongs to the admonition
        if let Some(title) = self.block_title.take() {
            admonition.title = title;
        }
        // the admonition holds the paragraph that follows (see `add_inlines_to_block_stack`), or
        // the delimited block (see `open_pending_admonition`). It isn't pushed onto the block
        // stack yet, so that a paragraph in a list item is still the item's
        self.pending_admonition = Some(admonition);
        Ok(())
    }

    /// `[NOTE]` directly above a list has no paragraph to hold (its paragraph, if any, has just
    /// been completed), so it is dropped; Asciidoctor makes it a role on the list, and we don't
    /// support roles in that shorthand form (see LIMITATIONS)
    pub(super) fn drop_admonition_above_list(&mut self) {
        self.pending_admonition = None;
    }

    /// An admonition with nothing after it (`NOTE:` then a blank line) is empty
    pub(super) fn add_empty_admonition(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
        match self.pending_admonition.take() {
            Some(admonition) => {
                self.add_to_block_stack_or_graph(asg, Block::ParentBlock(admonition))
            }
            None => Ok(()),
        }
    }

    /// `[NOTE]` above a delimited block: the admonition holds the block, once it is complete (see
    /// `wrap_in_admonition`). It stays off the block stack, so that a list item the block is
    /// attached to is still the open one when the block closes.
    fn open_pending_admonition(&mut self) {
        if let Some(admonition) = self.pending_admonition.take() {
            self.admonitions_around_blocks
                .push((self.block_stack.len(), admonition));
        }
    }

    /// A delimited block that is complete goes into the admonition opened with it, if any (its
    /// depth is the block stack's once the block is off it)
    pub(super) fn wrap_in_admonition(&mut self, block: Block) -> Result<Block, ParserError> {
        match self.admonitions_around_blocks.last() {
            Some((depth, _)) if *depth == self.block_stack.len() => {
                let Some((_, admonition)) = self.admonitions_around_blocks.pop() else {
                    return Ok(block);
                };
                let mut admonition = Block::ParentBlock(admonition);
                admonition.push_block(block)?;
                Ok(admonition)
            }
            _ => Ok(block),
        }
    }

    pub(super) fn parse_block_image(
        &mut self,
        token: Token,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        let (target, metadata) = target_and_attrs_from_token(&token)?;
        // like Asciidoctor, a missing image is a problem to report, not a reason to stop: the
        // block is still rendered with the target as written
        if let Err(e) = self.check_target(token.line, &target) {
            warn!("{e}");
        }
        let mut image_block = BlockMacro::new_image_block(target, metadata, token.locations());
        if let Some(metadata) = &self.metadata {
            // TODO see if there is a cleaner way to manage the borrowing here.
            image_block = image_block.add_metadata(metadata);
            self.metadata = None;
        }
        if let Some(caption) = &self.block_title {
            image_block.caption = caption.clone();
            self.block_title = None
        }
        self.add_to_block_stack_or_graph(asg, Block::BlockMacro(image_block))
    }

    pub(super) fn parse_delimited_leaf_block(
        &mut self,
        token: Token,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        if self.open_parse_after_as_text_type.is_some() {
            // ensure inlines are added appropriately
            self.add_inlines_to_block_stack()?;
            match self.block_stack.pop() {
                Some(mut open_leaf) => {
                    open_leaf.add_locations(token.locations().clone());
                    self.complete_block(open_leaf, asg)?;
                    self.open_parse_after_as_text_type = None;
                    Ok(())
                }
                None => Err(ParserError::OpenParse(token.line)),
            }
        } else {
            self.open_parse_after_as_text_type = Some(token.token_type());
            let mut block = LeafBlock::new_from_token(token)?;
            // a pending block title belongs to this block (and must not linger, or it will
            // swallow the inlines that follow)
            if let Some(title) = self.block_title.take() {
                block.title = title;
            }
            // the block is open (it takes the text that follows), so even after a `+` it stays
            // on the stack until it closes; only then is it attached
            self.open_pending_admonition();
            self.push_block_to_stack(Block::LeafBlock(block))?;
            // note that we're to just add
            self.force_new_block = false;
            Ok(())
        }
    }

    pub(super) fn parse_delimited_parent_block(
        &mut self,
        token: Token,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        let delimiter_line = token.first_location().line;
        let mut block = ParentBlock::new_from_token(token)?;
        // clear the dangling newline
        self.dangling_newline = None;

        if self.block_title.is_some() {
            block.title = self.block_title.as_ref().unwrap().clone();
            self.block_title = None;
        }

        // the innermost delimited block that is still open: the one that opened on the line (in
        // the file) `open_delimited_block_lines` ends with (other parent blocks on the stack, a
        // sidebar that has closed inside an example, say, are complete, and waiting for the
        // example to close)
        let open_line = self.open_delimited_block_lines.last().copied();
        let open_file = self.open_delimited_block_files.last().cloned();
        if let Some(parent_block_idx) = open_line.and_then(|line| {
            self.block_stack.iter().rposition(|parent_block| {
                matches!(parent_block, Block::ParentBlock(open)
                if open.location.first().is_some_and(|start| {
                    start.line == line && Some(&start.file) == open_file.as_ref()
                }))
            })
        }) {
            let closes_open_block = matches!(
                &self.block_stack[parent_block_idx],
                Block::ParentBlock(open) if *open == block
            );
            if closes_open_block {
                // close any dangling inlines BEFORE opening the delimited block lines (while the
                // open block is still on the stack, so they can't go to a list outside it)
                self.add_inlines_to_block_stack()?;
                // the closing delimiter normally closes the lists inside the block already; any
                // left open (inside a block that was never closed, say) are collected with the
                // rest of the block's contents
                for (depth, list) in self.lists.close_deeper_than(parent_block_idx)? {
                    let position = depth.min(self.block_stack.len());
                    self.block_stack.insert(position, list);
                }
                let Block::ParentBlock(mut matched) = self.block_stack.remove(parent_block_idx)
                else {
                    return Err(ParserError::ParentBlock(delimiter_line));
                };
                // remove the open delimiter line from the count and confirm we're nested properly
                let Some(line) = self.open_delimited_block_lines.pop() else {
                    return Err(ParserError::DelimitedBlock(matched.opening_line()?));
                };
                self.open_delimited_block_files.pop();
                if line != matched.opening_line()? {
                    warn!("Error nesting delimited blocks, see line {}", line)
                }
                // update the final location
                matched.location = Location::reconcile(matched.location.clone(), block.location);
                // collect subsequent blocks to be added to the parent block
                let mut blocks_to_add =
                    VecDeque::from_iter(self.block_stack.drain(parent_block_idx..));
                //
                let mut delimited_block = Block::ParentBlock(matched);
                while !blocks_to_add.is_empty() {
                    delimited_block.push_block(blocks_to_add.pop_front().unwrap())?
                }
                if delimited_block.is_table() {
                    delimited_block.consolidate_table_info()?;
                }
                self.complete_block(delimited_block, asg)?;
                return Ok(());
            }
        }
        // note the open block line
        self.open_delimited_block_lines.push(delimiter_line);
        self.open_delimited_block_files.push(
            block
                .location
                .first()
                .map(|start| start.file.clone())
                .unwrap_or_default(),
        );
        // the attributes above are the admonition's, if there is one
        if let Some(admonition) = &mut self.pending_admonition
            && let Some(metadata) = self.metadata.take()
        {
            admonition.metadata = Some(metadata).filter(|metadata| !metadata.is_empty());
        }
        self.open_pending_admonition();
        self.push_block_to_stack(Block::ParentBlock(block))
    }

    /// Whether the innermost open delimited block is a table
    pub(super) fn in_open_table(&self) -> bool {
        let (Some(line), Some(file)) = (
            self.open_delimited_block_lines.last(),
            self.open_delimited_block_files.last(),
        ) else {
            return false;
        };
        self.block_stack.iter().rev().any(|block| {
            block.is_table()
                && block
                    .locations()
                    .first()
                    .is_some_and(|start| start.line == *line && &start.file == file)
        })
    }

    pub(super) fn parse_table_cell(
        &mut self,
        token: Token,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        // take the token text, which begins with a `|`, and then use everything after
        let cell_contents = token.text()[1..].replace("\\|", "|");
        let cell_line = token.first_location().line;
        let cell_col = token.first_location().col;

        // text on the lines since the last cell is the rest of that cell's
        if matches!(self.block_stack.last(), Some(Block::TableCell(_))) {
            self.add_inlines_to_block_stack()?;
        }
        self.push_block_to_stack(Block::TableCell(TableCell::new_from_token(token)))?;

        // create new inlines from the stack, clearing any dangling newlines
        self.dangling_newline = None;
        for result in Scanner::new(&cell_contents) {
            match result {
                Ok(mut inline_token) => {
                    // update the line numbering for the token to match the table cell mathematically, in
                    // case my vague memory of multi-line cells is correct
                    inline_token.update_token_loc_offsets_by(cell_line, cell_col);
                    // the cell is scanned as if it started a line, but its text is only text
                    // (see `token_into`), a `|` that was escaped included
                    if inline_token.token_type() == TokenType::TableCell {
                        inline_token.make_text();
                    }
                    self.token_into(inline_token, asg)?;
                }
                Err(e) => return Err(ParserError::Scanner(e)),
            }
        }
        // clear all the things to ensure inlines get added appropriately
        self.in_block_line = false;
        self.force_new_block = false;
        // then add them to the stack, i.e., to the recently added TableCell
        self.add_inlines_to_block_stack()?;
        Ok(())
    }
}
