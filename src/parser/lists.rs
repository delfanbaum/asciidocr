//! Lists and description lists: their items, terms and continuations (what's open is kept in
//! [`list_context`](super::list_context))

use std::collections::VecDeque;

use crate::errors::ParserError;
use crate::graph::{
    asg::Asg,
    blocks::Block,
    inlines::{Inline, InlineLiteralName},
    lists::{DList, DListItem, List, ListItem, ListVariant},
    metadata::ElementMetadata,
    nodes::Location,
};
use crate::scanner::tokens::{Token, TokenType};

use super::Parser;
use super::list_context::marker_key;

impl Parser {
    /// After a blank line in a list, the next line that isn't blank decides whether the list
    /// goes on: an item or a `+` continues it, and anything else ends it. Ending it then does
    /// what the blank line would have done without a list.
    ///
    /// A line of text is only known not to be a description list item (whose terms come before
    /// the `::`) once it ends without a `::`, so text waits on the inline stack until then.
    pub(super) fn end_list_after_blank_line(
        &mut self,
        token_type: TokenType,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        if self
            .lists
            .active(self.block_stack.len())
            .is_none_or(|scope| !scope.after_blank)
        {
            return Ok(());
        }
        match token_type {
            // the end of a line of text: the text wasn't a term, so it comes after the list
            TokenType::NewLineChar | TokenType::Eof if !self.inline_stack.is_empty() => {
                let line = std::mem::take(&mut self.inline_stack);
                self.close_lists(asg)?;
                self.end_blocks_at_blank_line(asg)?;
                self.inline_stack = line;
            }
            // another blank line
            TokenType::NewLineChar
            | TokenType::BlockContinuation
            | TokenType::UnorderedListItem
            | TokenType::OrderedListItem
            | TokenType::CodeCalloutListItem
            | TokenType::DescriptionListMarker
            // an include decides nothing; the first line it includes does
            | TokenType::Include
            | TokenType::StartTag
            | TokenType::EndTag => {}
            // lines that aren't text
            token_type
                if token_type.ends_list_item()
                    || matches!(
                        token_type,
                        TokenType::Eof
                            | TokenType::Comment
                            | TokenType::Attribute
                            | TokenType::TableCell
                    ) =>
            {
                self.close_lists(asg)?;
                self.end_blocks_at_blank_line(asg)?;
            }
            // text, which may be the terms of a description list item
            _ => {}
        }
        Ok(())
    }

    /// Starts a description list item, which works like any other list item (see
    /// [`parse_list_item`](Self::parse_list_item)). Its terms are the text on the marker's line;
    /// the text on the lines before it belongs to what came before (the previous item, say).
    pub(super) fn parse_description_list_term(
        &mut self,
        token: Token,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        let depth = self.block_stack.len();
        // like a list item, a term can't interrupt a paragraph: if the paragraph started on an
        // earlier line, `Term::` is more of its text
        if self.lists.get(depth).is_none() && self.in_paragraph() && self.text_before_this_line() {
            return self.parse_text(token);
        }
        self.dangling_newline = None;
        // ensure we're not forcing a new block, given that all sorts of things can follow
        self.force_new_block = false;
        // each marker (`::`, `:::`, `::::`, `;;`) is its own list, as with other lists
        let key = token.lexeme.trim().to_string();
        let mut term = Vec::from(self.take_current_line_inlines());
        // like Asciidoctor, the term's indentation is not part of it
        if let Some(Inline::InlineLiteral(first)) = term.first_mut()
            && matches!(first.name, InlineLiteralName::Text)
        {
            first.trim_start();
        }

        self.add_inlines_to_block_stack()?;
        self.drop_admonition_above_list();
        // preserve newlines for now
        self.preserve_newline_text = true;
        // a term directly after a term is another term for the same description
        let term = match self.lists.active(depth) {
            Some(scope) => match scope.add_term(&key, term) {
                Some(term) => term,
                None => return Ok(()),
            },
            None => term,
        };
        let mut dlist_item = DListItem::new_from_token(token);
        dlist_item.add_term(term);
        let locations = dlist_item.locations();
        let item = Block::DListItem(dlist_item);

        if let Some(scope) = self.lists.active(depth) {
            if let Some(item) = scope.add_item(&key, item)? {
                let dlist = self.new_dlist(key.clone(), locations);
                if let Some(scope) = self.lists.active(depth) {
                    scope.nest(key, dlist, item);
                }
            }
            return Ok(());
        }

        self.emit_completed_block(asg)?;
        let dlist = self.new_dlist(key.clone(), locations);
        self.lists
            .start_list(self.block_stack.len(), key, dlist, item);
        Ok(())
    }

    /// Makes a description list, which takes the block title and metadata that are waiting for
    /// a block
    fn new_dlist(&mut self, marker: String, locations: Vec<Location>) -> Block {
        let mut dlist = DList::new(marker, locations);
        if let Some(title) = self.block_title.take() {
            dlist.title = title;
        }
        if let Some(metadata) = self.metadata.take()
            && !metadata.is_empty()
        {
            dlist.metadata = Some(metadata);
        }
        Block::DList(dlist)
    }

    /// Whether the inline stack holds text from the lines before this one (a newline with nothing
    /// before it, such as the one after a block anchor line, doesn't count)
    fn text_before_this_line(&self) -> bool {
        let mut seen_text = false;
        for inline in &self.inline_stack {
            if inline.is_newline() {
                if seen_text {
                    return true;
                }
                continue;
            }
            if let Inline::InlineLiteral(literal) = inline {
                let value = literal.value();
                if let Some(newline_idx) = value.find('\n')
                    && (seen_text || newline_idx > 0)
                {
                    return true;
                }
                seen_text |= !value.trim().is_empty();
            } else {
                seen_text = true;
            }
        }
        false
    }

    /// Takes the inlines on the current line off the inline stack: those after the last newline,
    /// which is dropped
    fn take_current_line_inlines(&mut self) -> VecDeque<Inline> {
        let mut line = VecDeque::new();
        while let Some(inline) = self.inline_stack.pop_back() {
            if inline.is_newline() {
                break;
            }
            // text that runs onto this line from the lines before is split at the newline
            if let Inline::InlineLiteral(mut literal) = inline {
                if matches!(literal.name, InlineLiteralName::Text)
                    && let Some(last_line) = literal.split_off_last_line()
                {
                    line.push_front(Inline::InlineLiteral(last_line));
                    if !literal.value().is_empty() {
                        self.inline_stack.push_back(Inline::InlineLiteral(literal));
                    }
                    break;
                }
                line.push_front(Inline::InlineLiteral(literal));
            } else {
                line.push_front(inline);
            }
        }
        line
    }

    /// Starts a list item: a new item in the open list with the same marker, or else a new list,
    /// nested in the current item if a list is open. Whatever text came before (the previous
    /// item's, or a section title) is finished first, so that it stays where it belongs.
    pub(super) fn parse_list_item(
        &mut self,
        token: Token,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        // clear any dangling newlines
        self.dangling_newline = None;
        let depth = self.block_stack.len();
        let marker = token.lexeme.trim().to_string();
        let key = marker_key(&marker);
        let item = Block::ListItem(ListItem::new(token.lexeme.clone(), token.locations()));
        // the item's text follows on this line even if an attribute line or a section heading
        // asked for a new block
        self.force_new_block = false;

        // the previous item's text (or the paragraph attached to it), or whatever came before the
        // list (e.g., a section title), is complete
        self.add_inlines_to_block_stack()?;
        self.drop_admonition_above_list();
        if let Some(scope) = self.lists.active(depth) {
            if let Some(item) = scope.add_item(&key, item)? {
                let list = Block::List(self.new_list(&token, marker, asg)?);
                if let Some(scope) = self.lists.active(depth) {
                    scope.nest(key, list, item);
                }
            }
            return Ok(());
        }

        self.emit_completed_block(asg)?;
        let list = Block::List(self.new_list(&token, marker, asg)?);
        self.lists
            .start_list(self.block_stack.len(), key, list, item);
        Ok(())
    }

    /// Makes the list that the item `token` starts. The list takes the block title and metadata
    /// that are waiting for a block
    fn new_list(
        &mut self,
        token: &Token,
        marker: String,
        asg: &mut Asg,
    ) -> Result<List, ParserError> {
        let variant = match token.token_type() {
            TokenType::UnorderedListItem => ListVariant::Unordered,
            _ => ListVariant::Ordered,
        };
        let mut list = List::new(variant, marker, token.locations());
        if token.token_type() == TokenType::CodeCalloutListItem {
            // check to see if we ought to "close" the source block (almost always)
            // TODO source blocks should close themselves, I think.
            if let Some(block) = self.block_stack.last()
                && block.is_source_block()
            {
                // we need to add this before we create the new list
                self.add_last_to_block_stack_or_graph(asg)?;
            }
            list.metadata = Some(ElementMetadata::new_with_role(
                "colist".to_string(),
                self.line,
            ));
        }
        if let Some(title) = self.block_title.take() {
            list.title = title;
        }
        if let Some(metadata) = self.metadata.take()
            && !metadata.is_empty()
        {
            list.metadata = Some(metadata);
        }
        Ok(list)
    }

    /// Called at a token that starts a block. If a `+` came just before, the block belongs to the
    /// current list item, which takes no more text. Otherwise the item's text (or the paragraph
    /// attached to it) is complete, and so is the list.
    pub(super) fn end_list_item_at_block(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
        let inlines_pending = !self.inline_stack.is_empty();
        let Some(scope) = self.lists.active(self.block_stack.len()) else {
            return Ok(());
        };
        // the newline that ended the item's last line of text is not the block's
        self.dangling_newline = None;
        if scope.attach_next && !inlines_pending {
            scope.principal_open = false;
            return Ok(());
        }
        self.add_inlines_to_block_stack()?;
        self.close_lists(asg)
    }

    /// Closes the lists at the current depth of the block stack, and puts the list where a
    /// completed block goes: into its parent (or the graph), or, inside a delimited block, onto
    /// the stack to wait for the closing delimiter.
    pub(super) fn close_lists(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
        let Some(list) = self.lists.close_active(self.block_stack.len())? else {
            return Ok(());
        };
        if !self.open_delimited_block_lines.is_empty() {
            self.block_stack.push(list);
        } else if let Some(parent) = self.block_stack.last_mut()
            && parent.takes_block_of_type(&list)
        {
            parent.push_block(list)?;
        } else {
            asg.push_block(list)?;
        }
        Ok(())
    }

    pub(super) fn parse_block_continuation(&mut self, _token: Token) -> Result<(), ParserError> {
        self.add_inlines_to_block_stack()?;
        if let Some(scope) = self.lists.active(self.block_stack.len()) {
            // the next block belongs to the current list item
            scope.continuation()?;
        } else {
            // outside a list, where Asciidoctor keeps a `+` as text; see the roadmap
            self.in_block_continuation = true;
        }
        self.force_new_block = true;
        Ok(())
    }

    /// A block that is complete while a `+` waits for one belongs to the current list item.
    /// Returns the block if no item is waiting for it.
    pub(super) fn attach_to_list_item(
        &mut self,
        block: Block,
    ) -> Result<Option<Block>, ParserError> {
        match self.lists.active(self.block_stack.len()) {
            Some(scope) if scope.attach_next => {
                scope.attach_block(block)?;
                Ok(None)
            }
            _ => Ok(Some(block)),
        }
    }
}
