use core::panic;
use std::collections::{HashMap, VecDeque};

use crate::{
    asg::Asg,
    blocks::{Block, Break, LeafBlock, Section},
    inlines::{Inline, InlineLiteral, InlineSpan},
    lists::{List, ListItem, ListVariant},
    nodes::{Header, Location},
    tokens::{Token, TokenType},
};

pub struct Parser {
    last_token_type: TokenType,
    document_header: Header,
    document_attributes: HashMap<String, String>,
    open_blocks: Vec<Block>,
    open_inlines: VecDeque<Inline>,
    in_document_header: bool,
    /// designates whether we're to be adding inlines to the previous block until a newline
    in_block_line: bool,
    /// designates whether new literal text should be added to the last span
    in_inline_span: bool,
    /// designates whether or not we want to keep a "section" block open to accept new blocks
    in_section: bool,
    /// forces a new block when we add inlines; helps distinguish between adding to section.title
    /// and section.blocks
    force_new_block: bool,
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
            open_inlines: VecDeque::new(),
            in_document_header: true,
            in_block_line: false,
            in_inline_span: false,
            in_section: false,
            force_new_block: false,
            dangling_newline: None,
        }
    }

    pub fn parse<I>(&mut self, tokens: I) -> Asg
    where
        I: Iterator<Item = Token>,
    {
        let mut asg = Asg::new();
        for token in tokens {
            let token_type = token.token_type();
            self.token_into(token, &mut asg);

            self.last_token_type = token_type;
        }

        // add any dangling inlines
        self.add_inlines_to_block_stack();
        // add any dangling blocks
        while !self.open_blocks.is_empty() {
            self.add_last_block_to_graph(&mut asg);
        }
        // cleanup the tree
        asg.consolidate();
        asg
    }

    fn token_into(&mut self, token: Token, asg: &mut Asg) {
        // if we are not starting with a document-heading acceptable token, get out
        if self.in_document_header && !token.can_be_in_document_header() {
            self.in_document_header = false;
        }

        match token.token_type() {
            // document header, headings and section parsing
            TokenType::Heading1 => self.parse_level_0_heading(token, asg),
            TokenType::Heading2 => self.parse_section_headings(token, asg, 1),
            TokenType::Heading3 => self.parse_section_headings(token, asg, 2),
            TokenType::Heading4 => self.parse_section_headings(token, asg, 3),
            TokenType::Heading5 => self.parse_section_headings(token, asg, 4),
            TokenType::Attribute => self.parse_attribute(token),

            // inlines
            TokenType::NewLineChar => self.parse_new_line_char(token, asg),
            TokenType::Text => self.parse_text(token),
            TokenType::Strong => self.parse_strong(token),
            TokenType::Emphasis => self.parse_emphasis(token),
            TokenType::Monospace => self.parse_code(token),
            TokenType::Mark => self.parse_mark(token),

            // breaks NEED TESTS
            TokenType::PageBreak => self.parse_page_break(token, asg),
            TokenType::ThematicBreak => self.parse_thematic_break(token, asg),

            // delimited blocks NEED TESTS
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

    /// New line characters are arguably the most significant token "signal" we can get, and as
    /// such the parse function is a little complicated.
    fn parse_new_line_char(&mut self, token: Token, asg: &mut Asg) {
        // newline exits a title, TK line continuation
        self.in_block_line = false;

        if [TokenType::NewLineChar, TokenType::Eof].contains(&self.last_token_type) {
            // clear any dangling newline
            self.dangling_newline = None;
            if self.in_document_header {
                if !self.document_header.is_empty() {
                    self.document_header.consolidate();
                    asg.add_header(
                        self.document_header.clone(),
                        self.document_attributes.clone(),
                    )
                }
                self.in_document_header = false
            } else {
                // clear out any inlines
                self.in_inline_span = false;
                self.add_inlines_to_block_stack();
                // and then force a new block hereafter
                self.force_new_block = true;
                // check for dangling list items
                if let Some(last_block) = self.open_blocks.pop() {
                    if matches!(last_block, Block::ListItem(_)) {
                        // sanity check
                        if let Some(mut next_last_block) = self.open_blocks.pop() {
                            if matches!(next_last_block, Block::List(_)) {
                                next_last_block.push_block(last_block);
                                self.add_to_block_stack_or_graph(asg, next_last_block);
                            } else {
                                panic!("Dangling list item");
                            }
                        } else {
                            panic!("Dangling list item");
                        }
                    } else {
                        self.add_to_block_stack_or_graph(asg, last_block);
                    }
                }
            }
        } else {
            self.dangling_newline = Some(token)
        }
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

    // Comments
    fn parse_comment(&self) {
        // for now, do nothing
    }
    fn parse_comment_block(&mut self) {
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

    fn parse_level_0_heading(&mut self, token: Token, asg: &mut Asg) {
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

    fn parse_section_headings(&mut self, token: Token, asg: &mut Asg, level: usize) {
        self.add_to_block_stack_or_graph(
            asg,
            Block::Section(Section::new("".to_string(), level, token.first_location())),
        );
        // let us know we're in a block line
        self.in_block_line = true;
        // let us know that we want to add to the section title for a little bit
        self.force_new_block = false;
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

    fn parse_strong(&mut self, token: Token) {
        let inline = Inline::InlineSpan(InlineSpan::new_strong_span(token));
        if let Some(last_inline) = self.open_inlines.back_mut() {
            if inline == *last_inline {
                last_inline.reconcile_locations(inline.locations());
                self.in_inline_span = false;
                return;
            }
        }
        self.in_inline_span = true;
        self.open_inlines.push_back(inline)
    }

    fn parse_emphasis(&mut self, token: Token) {
        let inline = Inline::InlineSpan(InlineSpan::new_emphasis_span(token));
        if let Some(last_inline) = self.open_inlines.back_mut() {
            if inline == *last_inline {
                last_inline.reconcile_locations(inline.locations());
                self.in_inline_span = false;
                return;
            }
        }
        self.in_inline_span = true;
        self.open_inlines.push_back(inline)
    }

    fn parse_code(&mut self, token: Token) {
        let inline = Inline::InlineSpan(InlineSpan::new_code_span(token));
        if let Some(last_inline) = self.open_inlines.back_mut() {
            if inline == *last_inline {
                last_inline.reconcile_locations(inline.locations());
                self.in_inline_span = false;
                return;
            }
        }
        self.in_inline_span = true;
        self.open_inlines.push_back(inline)
    }

    fn parse_mark(&mut self, token: Token) {
        let inline = Inline::InlineSpan(InlineSpan::new_mark_span(token));
        if let Some(last_inline) = self.open_inlines.back_mut() {
            if inline == *last_inline {
                last_inline.reconcile_locations(inline.locations());
                self.in_inline_span = false;
                return;
            }
        }
        self.in_inline_span = true;
        self.open_inlines.push_back(inline)
    }

    //fn parse_superscript(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_subscript(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_highlighted(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_link_macro(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_footnote_macro(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_passthrough_inline_macro(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_inline_macro_close(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_eof(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_inline_style(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_aside_block(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_quote_verse_block(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_admonition_block(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_open_block(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_text(&mut self, token: Token) {
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
                self.add_text_to_last_inline(newline_token);
                self.dangling_newline = None;
            }
            self.add_text_to_last_inline(token)
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

    fn add_text_to_last_inline(&mut self, token: Token) {
        let inline_literal = Inline::InlineLiteral(InlineLiteral::new_text_from_token(&token));
        if let Some(last_inline) = self.open_inlines.back_mut() {
            match last_inline {
                Inline::InlineSpan(span) => {
                    if self.in_inline_span {
                        span.add_inline(inline_literal);
                    } else {
                        self.open_inlines.push_back(inline_literal)
                    }
                }
                Inline::InlineLiteral(prior_literal) => prior_literal.add_text_from_token(&token),
                Inline::InlineRef(_) => todo!(),
            }
        } else {
            self.open_inlines.push_back(inline_literal)
        }
    }

    fn add_inlines_to_block_stack(&mut self) {
        if self.in_inline_span {
            // TK HANDLE DANGLING ITALICS
            todo!()
        }
        if let Some(last_block) = self.open_blocks.last_mut() {
            if last_block.takes_inlines() && !self.in_block_line && !self.force_new_block {
                while !self.open_inlines.is_empty() {
                    if let Some(inline) = self.open_inlines.pop_front() {
                        last_block.push_inline(inline)
                    }
                }
                return;
            }
        }
        // create a new para from the locations of the first span (subsequent locations are
        // consolidated later)
        let mut para_locations: Vec<Location> = Vec::new();
        if let Some(first_inline) = self.open_inlines.front() {
            para_locations = first_inline.locations().clone();
        }
        let mut para_block = Block::LeafBlock(LeafBlock::new(
            crate::blocks::LeafBlockName::Paragraph,
            crate::blocks::LeafBlockForm::Paragraph,
            None,
            para_locations,
            vec![],
        ));
        while !self.open_inlines.is_empty() {
            if let Some(inline) = self.open_inlines.pop_front() {
                para_block.push_inline(inline)
            }
        }
        self.open_blocks.push(para_block)
    }

    fn add_last_list_item_to_list(&mut self) {
        // add the inlines to the list item
        self.add_inlines_to_block_stack();
        // then add it to the list
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

    fn add_to_block_stack_or_graph(&mut self, asg: &mut Asg, block: Block) {
        //if block.is_section() {
        //    block.create_id()
        //}
        if let Some(last_block) = self.open_blocks.last_mut() {
            if last_block.can_be_parent() {
                last_block.push_block(block)
            }
        } else if block.can_be_parent() {
            // and is closed by its closing condition
            self.open_blocks.push(block)
        } else {
            if !self.in_section {
                asg.push_block(block)
            }
        }
    }

    fn add_last_block_to_graph(&mut self, asg: &mut Asg) {
        if let Some(block) = self.open_blocks.pop() {
            if let Some(section) = self.open_blocks.last_mut() {
                section.push_block(block)
            } else if matches!(block, Block::ListItem(_)) {
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
}
