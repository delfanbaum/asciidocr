use core::panic;
use std::collections::HashMap;

use crate::{
    asg::Asg,
    blocks::{Block, Break, LeafBlock, Section},
    inlines::{Inline, InlineLiteral},
    lists::{List, ListItem, ListVariant},
    nodes::Header,
    tokens::{Token, TokenType},
};

pub struct Parser {
    last_token_type: TokenType,
    document_header: Header,
    document_attributes: HashMap<String, String>,
    open_blocks: Vec<Block>,
    _open_inlines: Vec<Inline>,
    in_document_header: bool,
    /// designates whether we're to be adding inlines to the previous block until a newline
    in_block_line: bool,
    token_count: usize,
    // used to see if we need to add a newline before new text
    dangling_newline: Option<Token>,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            last_token_type: TokenType::Eof,
            document_header: Header::new(),
            document_attributes: HashMap::new(),
            open_blocks: vec![],
            _open_inlines: vec![],
            in_document_header: true,
            in_block_line: false,
            token_count: 0,
            dangling_newline: None,
        }
    }

    pub fn parse<I>(&mut self, tokens: I) -> Asg
    where
        I: Iterator<Item = Token>,
    {
        let mut asg = Asg::new();
        for token in tokens {
            println!("{:?}", token);
            let token_type = token.token_type();
            self.token_into(token, &mut asg);

            self.last_token_type = token_type;
            self.token_count += 1;
        }

        while !self.open_blocks.is_empty() {
            if let Some(block) = self.open_blocks.pop() {
                if matches!(block, Block::ListItem(_)) {
                    // sanity check
                    if let Some(mut next_last_block) = self.open_blocks.pop() {
                        if matches!(next_last_block, Block::List(_)) {
                            next_last_block.push_block(block);
                            asg.push_block(next_last_block);
                        } else {
                            self.open_blocks.push(next_last_block)
                        }
                    }
                } else {
                    asg.push_block(block)
                }
            }
        }
        // TODO get last location in tree -- and other cleanup, probably an asg.consolidate() or
        // something
        asg.consolidate();
        asg
    }

    fn token_into(&mut self, token: Token, asg: &mut Asg) {
        // if we are not starting with a document-heading acceptable token, get out
        if self.in_document_header && !token.can_be_in_document_header() {
            self.in_document_header = false;
        }

        match token.token_type() {
            // document header and heading things
            TokenType::Heading1 => self.parse_heading1(token, asg),
            TokenType::Heading2 => self.parse_heading2(token, asg),
            TokenType::Heading3 => self.parse_heading3(token, asg),
            TokenType::Heading4 => self.parse_heading4(token, asg),
            TokenType::Heading5 => self.parse_heading5(token, asg),
            TokenType::Attribute => self.parse_attribute(token),

            // inlines
            TokenType::Text => self.parse_text(token, asg),
            TokenType::NewLineChar => self.parse_new_line_char(token, asg),

            // breaks
            TokenType::PageBreak => self.parse_page_break(token, asg),
            TokenType::ThematicBreak => self.parse_thematic_break(token, asg),

            // delimited blocks
            TokenType::PassthroughBlock => self.parse_passthrough_block(token, asg),
            TokenType::SourceBlock => self.parse_source_block(token, asg),
            TokenType::Comment => self.parse_comment(),
            TokenType::CommentBlock => self.parse_comment_block(),

            // lists
            TokenType::UnorderedListItem => self.parse_unordered_list_item(token),
            TokenType::OrderedListItem => self.parse_ordered_list_item(token),

            _ => {}
        }
    }

    fn parse_attribute(&mut self, token: Token) {
        let binding = token.text();
        let mut attr_components: Vec<&str> = binding.split_terminator(":").collect();
        attr_components.remove(0); // throw away initial "" in the list
        if attr_components.is_empty() {
            // guard clause
            todo!()
        }
        let key = attr_components.first().unwrap().to_string();
        // values should be trimmed
        let mut value = attr_components.last().unwrap().trim().to_string();
        if key == value {
            value = String::from("")
        }
        self.document_attributes.insert(key, value);
    }

    fn parse_new_line_char(&mut self, token: Token, asg: &mut Asg) {
        if self.in_block_line {
            // newline exits a title, TK line continuation
            self.in_block_line = false;
        }
        if [TokenType::NewLineChar, TokenType::Eof].contains(&self.last_token_type)
            && self.in_document_header
        {
            if !self.document_header.is_empty() {
                self.document_header.consolidate();
                asg.add_header(
                    self.document_header.clone(),
                    self.document_attributes.clone(),
                )
            }
            self.in_document_header = false
        }
        if let Some(last_block) = self.open_blocks.pop() {
            if last_block.can_be_parent() {
                // only new sections/delimiter blocks can close parents, so we just add this back
                self.open_blocks.push(last_block);
            // but if it's not a parent and this is a double new line, put the block in place
            } else if self.last_token_type == TokenType::NewLineChar {
                // if we're the last in a list, we need to push into the list and also close the list
                if matches!(last_block, Block::ListItem(_)) {
                    // sanity check
                    if let Some(mut next_last_block) = self.open_blocks.pop() {
                        if matches!(next_last_block, Block::List(_)) {
                            next_last_block.push_block(last_block);
                            self.add_to_block_stack_or_graph(asg, next_last_block);
                        } else {
                            self.open_blocks.push(next_last_block)
                        }
                    }
                } else {
                    self.add_to_block_stack_or_graph(asg, last_block);
                }
                self.dangling_newline = None;
            // otherwise put it back on the block stack
            } else {
                self.open_blocks.push(last_block);
            }
        }
        // "else", add newline literal to the last block or create a block
        self.dangling_newline = Some(token)
    }

    fn parse_thematic_break(&mut self, token: Token, asg: &mut Asg) {
        // TK does this need to be after a blank line or comment?
        self.add_to_block_stack_or_graph(
            asg,
            Block::Break(Break::new(
                crate::blocks::BreakVariant::Thematic,
                token.locations(),
            )),
        )
    }

    fn parse_page_break(&mut self, token: Token, asg: &mut Asg) {
        // TK does this need to be after a blank line or comment?
        self.add_to_block_stack_or_graph(
            asg,
            Block::Break(Break::new(
                crate::blocks::BreakVariant::Page,
                token.locations(),
            )),
        )
    }

    fn parse_comment(&self) {
        // for now, do nothing
    }

    fn parse_passthrough_block(&mut self, token: Token, asg: &mut Asg) {
        let block = LeafBlock::new_pass(Some(token.text()), token.first_location());
        self.handle_delimited_leaf_block(asg, block)
    }

    fn parse_source_block(&mut self, token: Token, asg: &mut Asg) {
        let block = LeafBlock::new_listing(Some(token.text()), token.first_location());
        self.handle_delimited_leaf_block(asg, block)
    }

    //fn parse_aside_block(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_quote_verse_block(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_comment_block(&mut self) {
        // for now, do nothing
    }

    //fn parse_admonition_block(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_open_block(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_ordered_list_item(&mut self, token: Token) {
        // clear any dangling newlines
        self.dangling_newline = None;
        let list_item = ListItem::new(token.lexeme.clone(), token.locations());
        // if there is an appropriate list, we push this onto the open_blocks so inlines can be
        // added
        if self.open_blocks.last().is_some() && self.open_blocks.last().unwrap().is_ordered_list() {
            self.add_last_list_item_to_list()
        } else {
            // we need to create the list first
            self.open_blocks.push(Block::List(List::new(
                ListVariant::Ordered,
                token.locations().clone(),
            )));
        }
        // either way, add the new list item
        self.open_blocks.push(Block::ListItem(list_item));
    }
    //
    fn parse_unordered_list_item(&mut self, token: Token) {
        // clear any dangling newlines
        self.dangling_newline = None;
        let list_item = ListItem::new(token.lexeme.clone(), token.locations());
        // if there is an appropriate list, we push this onto the open_blocks so inlines can be
        // added
        if self.open_blocks.last().is_some() && self.open_blocks.last().unwrap().is_unordered_list()
        {
            self.add_last_list_item_to_list()
        } else {
            // we need to create the list first
            self.open_blocks.push(Block::List(List::new(
                ListVariant::Unordered,
                token.locations().clone(),
            )));
        }
        // either way, add the new list item
        self.open_blocks.push(Block::ListItem(list_item));
    }

    //fn parse_block_label(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_heading1(&mut self, token: Token, asg: &mut Asg) {
        if self.in_document_header {
            self.document_header.location.extend(token.locations());
            self.in_block_line = true;
        } else {
            if let Some(last_block) = self.open_blocks.pop() {
                match last_block {
                    Block::Section(section) => {
                        if section.level == 1 {
                            self.add_to_block_stack_or_graph(asg, Block::Section(section))
                        } else {
                            self.open_blocks.push(Block::Section(section))
                        }
                    }
                    _ => self.open_blocks.push(last_block),
                }
            }
            self.add_to_block_stack_or_graph(
                asg,
                Block::Section(Section::new("".to_string(), 1, token.first_location())),
            );
        }
    }

    fn parse_heading2(&mut self, token: Token, asg: &mut Asg) {
        self.add_to_block_stack_or_graph(
            asg,
            Block::Section(Section::new("".to_string(), 2, token.first_location())),
        );
    }
    fn parse_heading3(&mut self, token: Token, asg: &mut Asg) {
        self.add_to_block_stack_or_graph(
            asg,
            Block::Section(Section::new("".to_string(), 3, token.first_location())),
        );
    }
    fn parse_heading4(&mut self, token: Token, asg: &mut Asg) {
        self.add_to_block_stack_or_graph(
            asg,
            Block::Section(Section::new("".to_string(), 4, token.first_location())),
        );
    }
    fn parse_heading5(&mut self, token: Token, asg: &mut Asg) {
        self.add_to_block_stack_or_graph(
            asg,
            Block::Section(Section::new("".to_string(), 5, token.first_location())),
        );
    }

    //fn parse_blockquote(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_verse(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_source(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_note(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_tip(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_important(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_caution(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_warning(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_block_continuation(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_def_list_mark(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_bold(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_italic(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_monospace(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_superscript(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_subscript(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_highlighted(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_link_macro(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_footnote_macro(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_passthrough_inline_macro(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_inline_macro_close(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_eof(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_inline_style(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_text(&mut self, token: Token, asg: &mut Asg) {
        if self.in_document_header && self.in_block_line {
            if let Some(inline) = self.document_header.title.last_mut() {
                match inline {
                    Inline::InlineLiteral(lit) => lit.add_text_from_token(&token),
                    Inline::InlineSpan(span) => span.add_inline(Inline::InlineLiteral(
                        InlineLiteral::new_text_from_token(&token),
                    )),
                    Inline::InlineRef(_) => {
                        panic!("Inline references are not allowed in document titles")
                    }
                }
            } else {
                self.document_header.title.push(Inline::InlineLiteral(
                    InlineLiteral::new_text_from_token(&token),
                ));
            }
        } else {
            if let Some(newline_token) = self.dangling_newline.clone() {
                self.add_text_to_last_inline(asg, newline_token);
                self.dangling_newline = None;
            }
            self.add_text_to_last_inline(asg, token)
        }
    }

    fn handle_delimited_leaf_block(&mut self, asg: &mut Asg, block: LeafBlock) {
        if let Some(open_block) = self.open_blocks.last_mut() {
            match open_block {
                Block::LeafBlock(leaf) => {
                    if leaf.name == block.name {
                        self.close_last_open_block(asg)
                    } else {
                        self.add_to_block_stack_or_graph(asg, Block::LeafBlock(block))
                    }
                }
                _ => self.open_blocks.push(Block::LeafBlock(block)),
            }
        }
    }

    fn close_last_open_block(&mut self, asg: &mut Asg) {
        if let Some(block) = self.open_blocks.pop() {
            self.add_to_block_stack_or_graph(asg, block)
        } else {
            panic!("Unexpected close last block call!")
        }
    }

    fn add_text_to_last_inline(&mut self, asg: &mut Asg, token: Token) {
        if let Some(last_block) = self.open_blocks.last_mut() {
            if let Some(last_inline) = last_block.last_inline() {
                match last_inline {
                    Inline::InlineLiteral(lit) => lit.add_text_from_token(&token),
                    Inline::InlineSpan(span) => span.add_inline(Inline::InlineLiteral(
                        InlineLiteral::new_text_from_token(&token),
                    )),
                    Inline::InlineRef(_) => todo!(),
                }
            } else if last_block.takes_inlines() {
                last_block.push_inline(Inline::InlineLiteral(InlineLiteral::new_text_from_token(
                    &token,
                )))
            } else {
                // newlines on their own don't get a paragraph
                if token.token_type() != TokenType::NewLineChar {
                    self.add_inline_to_block_stack(Inline::InlineLiteral(
                        InlineLiteral::new_text_from_token(&token),
                    ))
                } else {
                    self.add_last_block_to_graph(asg);
                }
            }
        } else if token.token_type() != TokenType::NewLineChar {
            self.add_inline_to_block_stack(Inline::InlineLiteral(
                InlineLiteral::new_text_from_token(&token),
            ))
        }
    }

    fn add_inline_to_block_stack(&mut self, inline: Inline) {
        self.open_blocks.push(Block::LeafBlock(LeafBlock::new(
            crate::blocks::LeafBlockName::Paragraph,
            crate::blocks::LeafBlockForm::Paragraph,
            None,
            inline.locations(),
            vec![inline],
        )))
    }

    fn add_last_list_item_to_list(&mut self) {
        let last_item = self.open_blocks.pop().unwrap();
        // if the last thing is a list item, add it to the list
        if matches!(last_item, Block::ListItem(_)) {
            if let Some(list) = self.open_blocks.last_mut() {
                list.push_block(last_item)
            }
        } else {
            // otherwise return the list to the open block stack, and create a new unordered
            // list item
            self.open_blocks.push(last_item);
        }
    }

    fn add_to_block_stack_or_graph(&mut self, asg: &mut Asg, mut block: Block) {
        if block.is_section() {
            block.create_id()
        }
        if let Some(last_block) = self.open_blocks.last_mut() {
            if last_block.can_be_parent() {
                last_block.push_block(block)
            }
        } else if block.can_be_parent() {
            // and is closed by its closing condition
            self.open_blocks.push(block)
        } else {
            asg.push_block(block)
        }
    }

    fn add_last_block_to_graph(&mut self, asg: &mut Asg) {
        if let Some(block) = self.open_blocks.pop() {
            asg.push_block(block)
        }
    }
}
