use core::panic;
use std::collections::{HashMap, VecDeque};

use crate::{
    asg::Asg,
    blocks::{Block, Break, LeafBlock, ParentBlock, Section},
    inlines::{Inline, InlineLiteral, InlineRef, InlineSpan},
    lists::{List, ListItem, ListVariant},
    nodes::{Header, Location},
    tokens::{Token, TokenType},
};

/// Parses a stream of tokens into an Abstract Syntax Graph, returning the graph once all tokens
/// have been parsed.
pub struct Parser {
    last_token_type: TokenType,
    document_header: Header,
    document_attributes: HashMap<String, String>,
    /// holding ground for graph blocks until it's time to push to the main graph
    block_stack: Vec<Block>,
    /// holding ground for inline elements until it's time to push to the relevant block
    inline_stack: VecDeque<Inline>,
    /// holding ground for block metadata, to be applied to the subsequent block
    _block_metadata: Option<Block>,
    /// counts in/out delimited blocks by line reference; allows us to warn/error if they are
    /// unclosed at the end of the document
    open_delimited_block_lines: Vec<usize>,
    // convenience flags
    in_document_header: bool,
    /// designates whether we're to be adding inlines to the previous block until a newline
    in_block_line: bool,
    /// designates whether new literal text should be added to the last span
    in_inline_span: bool,
    /// designates whether, despite newline last_tokens_types, we should append the current block
    /// to the next
    in_block_continuation: bool,
    /// forces a new block when we add inlines; helps distinguish between adding to section.title
    /// and section.blocks
    force_new_block: bool,
    /// Some parent elements have non-obvious closing conditions, so we want an easy way to close these
    close_parent_after_push: bool,
    /// Used to see if we need to add a newline before new text; we don't add newlines to the text
    /// literals unless they're continuous (i.e., we never count newline paras as paras)
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
            block_stack: vec![],
            inline_stack: VecDeque::new(),
            _block_metadata: None,
            open_delimited_block_lines: vec![],
            in_document_header: true,
            in_block_line: false,
            in_inline_span: false,
            in_block_continuation: false,
            force_new_block: false,
            close_parent_after_push: false,
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
        while !self.block_stack.is_empty() {
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
            TokenType::AttributeReference => self.parse_attribute_reference(token),

            // refs
            TokenType::LinkMacro => self.parse_link_macro(token),
            TokenType::InlineMacroClose => self.parse_inline_macro_close(token),

            // breaks NEED TESTS
            TokenType::PageBreak => self.parse_page_break(token, asg),
            TokenType::ThematicBreak => self.parse_thematic_break(token, asg),

            // delimited blocks
            TokenType::SidebarBlock
            | TokenType::OpenBlock
            | TokenType::QuoteVerseBlock
            | TokenType::ExampleBlock => self.parse_delimited_block(token, asg),

            // the following should probably be consumed into the above
            TokenType::PassthroughBlock => self.parse_passthrough_block(token, asg),
            TokenType::SourceBlock => self.parse_source_block(token, asg),
            TokenType::CommentBlock => self.parse_comment_block(),

            // lists
            TokenType::UnorderedListItem => self.parse_unordered_list_item(token),
            TokenType::OrderedListItem => self.parse_ordered_list_item(token),

            // inline admonitions
            TokenType::NotePara
            | TokenType::TipPara
            | TokenType::ImportantPara
            | TokenType::CautionPara
            | TokenType::WarningPara => self.parse_admonition_para_syntax(token),

            // block continuation... I think does nothing, parser-wise, since it simply prevents
            // the double newline
            TokenType::BlockContinuation => self.parse_block_continuation(token),

            // comments
            TokenType::Comment => self.parse_comment(),

            _ => {
                // self check
                println!("\"{:?}\" not implemented", token.token_type());
                todo!()
            }
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
                if let Some(last_block) = self.block_stack.pop() {
                    if !last_block.is_section() && self.open_delimited_block_lines.is_empty() {
                        self.add_to_block_stack_or_graph(asg, last_block);
                        if self.close_parent_after_push {
                            self.add_last_to_block_stack_or_graph(asg);
                            self.close_parent_after_push = false;
                        }
                    } else {
                        self.push_block_to_stack(last_block)
                    }
                } // if Some(last_block)
            }
        } else if self.in_block_continuation {
            // don't add a newline ahead of text
            self.dangling_newline = None;
        } else {
            self.dangling_newline = Some(token);
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
        if self.block_stack.last().is_some()
            && self.block_stack.last().unwrap().is_ordered_list_item()
        {
            self.add_last_list_item_to_list()
        } else {
            // we need to create the list first
            self.push_block_to_stack(Block::List(List::new(
                ListVariant::Ordered,
                token.locations().clone(),
            )));
        }
        // either way, add the new list item
        self.push_block_to_stack(Block::ListItem(list_item));
    }
    //
    fn parse_unordered_list_item(&mut self, token: Token) {
        // clear any dangling newlines
        self.dangling_newline = None;
        let list_item = ListItem::new(token.lexeme.clone(), token.locations());
        // if there is an appropriate list, we push this onto the open_blocks so inlines can be
        // added
        if self.block_stack.last().is_some()
            && self.block_stack.last().unwrap().is_unordered_list_item()
        {
            self.add_last_list_item_to_list()
        } else {
            // we need to create the list first
            self.push_block_to_stack(Block::List(List::new(
                ListVariant::Unordered,
                token.locations().clone(),
            )));
        }
        // either way, add the new list item
        self.push_block_to_stack(Block::ListItem(list_item));
    }

    //fn parse_block_label(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_level_0_heading(&mut self, token: Token, asg: &mut Asg) {
        if self.in_document_header {
            self.document_header.location.extend(token.locations());
            self.in_block_line = true;
        } else {
            if let Some(last_block) = self.block_stack.pop() {
                match last_block {
                    Block::Section(section) => {
                        if section.level == 1 {
                            self.add_to_block_stack_or_graph(asg, Block::Section(section))
                        } else {
                            self.push_block_to_stack(Block::Section(section))
                        }
                    }
                    _ => self.push_block_to_stack(last_block),
                }
            }
            self.add_to_block_stack_or_graph(
                asg,
                Block::Section(Section::new("".to_string(), 1, token.first_location())),
            );
        }
    }

    fn parse_section_headings(&mut self, token: Token, asg: &mut Asg, level: usize) {
        // if the last block is a section of the same level, push it up
        if let Some(last_block) = self.block_stack.pop() {
            if last_block.level_check().unwrap_or(999) == level {
                self.add_to_block_stack_or_graph(asg, last_block)
            } else {
                self.push_block_to_stack(last_block)
            }
        }
        // always add new sections directly to the block stack
        self.push_block_to_stack(Block::Section(Section::new(
            "".to_string(),
            level,
            token.first_location(),
        )));
        // let us know we're in a block line
        self.in_block_line = true;
        // let us know that we want to add to the section title for a little bit
        self.force_new_block = false;
    }

    //fn parse_blockquote(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_verse(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_source(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_admonition_para_syntax(&mut self, token: Token) {
        self.block_stack
            .push(Block::ParentBlock(ParentBlock::new_from_token(token)));
        self.close_parent_after_push = true;
    }

    //fn parse_tip(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_important(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_caution(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_warning(&mut self, token: Token, asg: &mut Asg) {}
    fn parse_block_continuation(&mut self, _token: Token) {
        self.add_inlines_to_block_stack();
        self.in_block_continuation = true;
        self.force_new_block = true;
    }
    //fn parse_def_list_mark(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_strong(&mut self, token: Token) {
        let inline = Inline::InlineSpan(InlineSpan::new_strong_span(token));
        if let Some(last_inline) = self.inline_stack.back_mut() {
            if inline == *last_inline {
                last_inline.reconcile_locations(inline.locations());
                self.in_inline_span = false;
                return;
            }
        }
        self.in_inline_span = true;
        self.inline_stack.push_back(inline)
    }

    fn parse_emphasis(&mut self, token: Token) {
        let inline = Inline::InlineSpan(InlineSpan::new_emphasis_span(token));
        if let Some(last_inline) = self.inline_stack.back_mut() {
            if inline == *last_inline {
                last_inline.reconcile_locations(inline.locations());
                self.in_inline_span = false;
                return;
            }
        }
        self.in_inline_span = true;
        self.inline_stack.push_back(inline)
    }

    fn parse_code(&mut self, token: Token) {
        let inline = Inline::InlineSpan(InlineSpan::new_code_span(token));
        if let Some(last_inline) = self.inline_stack.back_mut() {
            if inline == *last_inline {
                last_inline.reconcile_locations(inline.locations());
                self.in_inline_span = false;
                return;
            }
        }
        self.in_inline_span = true;
        self.inline_stack.push_back(inline)
    }

    fn parse_mark(&mut self, token: Token) {
        let inline = Inline::InlineSpan(InlineSpan::new_mark_span(token));
        if let Some(last_inline) = self.inline_stack.back_mut() {
            if inline == *last_inline {
                last_inline.reconcile_locations(inline.locations());
                self.in_inline_span = false;
                return;
            }
        }
        self.in_inline_span = true;
        self.inline_stack.push_back(inline)
    }

    //fn parse_superscript(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_subscript(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_highlighted(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_link_macro(&mut self, token: Token) {
        self.inline_stack
            .push_back(Inline::InlineRef(InlineRef::new_link_from_token(token)))
    }

    //fn parse_footnote_macro(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_passthrough_inline_macro(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_inline_macro_close(&mut self, token: Token) {
        if let Some(inline_macro_idx) = self
            .inline_stack
            .iter()
            .rposition(|inline| inline.is_macro())
        {
            // consolidate into the inline macro
            let Some(mut inline_macro) = self.inline_stack.remove(inline_macro_idx) else {
                panic!("Index error getting inline macro")
            };
            while self.inline_stack.len() > inline_macro_idx {
                let Some(subsequent_inline) = self.inline_stack.remove(inline_macro_idx) else {
                    panic!("Index error while adding inlines to inline macro")
                };
                inline_macro.push_inline(subsequent_inline);
            }
            // update the locations
            inline_macro.consolidate_locations_from_token(token);
            // add it back to the stack
            self.inline_stack.push_back(inline_macro);
            // note that we're now closed
            self.close_parent_after_push = true;
        } else {
            self.add_text_to_last_inline(token);
        }
    }

    //fn parse_eof(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_inline_style(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_quote_verse_block(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_admonition_block(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_open_block(&mut self, token: Token, asg: &mut Asg) {}
    //

    /// attribute references become literals, but we need to replace them with the appropriate
    /// values from the document header first
    fn parse_attribute_reference(&mut self, mut token: Token) {
        // the "{attribute}"
        let attribute_ref = token.text();
        let attribute_target: &str = &attribute_ref[1..attribute_ref.len() - 1];
        // update the token value
        if let Some((_, value)) = self.document_attributes.get_key_value(attribute_target) {
            // update the values
            token.literal = Some(value.clone());
            // update the ending col, adding the new value and then subtracting one because of
            // indexing
            token.endcol = token.startcol + value.len() - 1;
        } else {
            // TODO throw a better warning
            eprintln!("Missing document attribute: {}", attribute_target);
        }
        // then add it as literal text
        self.parse_text(token);
    }

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

    // TODO: handle [NOTE]\n==== cases (i.e., some block metadata check)
    fn parse_delimited_block(&mut self, token: Token, asg: &mut Asg) {
        let delimiter_line = token.first_location().line;
        let block = ParentBlock::new_from_token(token);

        // check for any prior parents in reverse
        if let Some(parent_block_idx) = self
            .block_stack
            .iter()
            .rposition(|parent_block| matches!(parent_block, Block::ParentBlock(_)))
        {
            let matched_block = self.block_stack.remove(parent_block_idx);
            let Block::ParentBlock(mut matched) = matched_block else {
                panic!("Unexpteced block in Block::ParentBlock")
            };
            if matched == block {
                // remove the open delimiter line from the count and confirm we're nested properly
                let Some(line) = self.open_delimited_block_lines.pop() else {
                    panic!("Attempted to close a non-existent delimited block")
                };
                if line != matched.opening_line() {
                    // TODO this should be an error, not a panic
                    panic!("Error nesting delimited blocks, see line {}", line)
                }
                // update the final location
                matched.location = Location::reconcile(matched.location.clone(), block.location);
                // return the parent block
                self.block_stack
                    .insert(parent_block_idx, Block::ParentBlock(matched));
                // close any dangling inlines
                self.add_inlines_to_block_stack();
                // add blocks until we have also added the parent block
                while self.block_stack.len() > parent_block_idx {
                    self.close_last_open_block(asg);
                }
                return;
            } else {
                self.block_stack
                    .insert(parent_block_idx, Block::ParentBlock(matched));
            }
        }
        // note the open block line
        self.open_delimited_block_lines.push(delimiter_line);
        self.push_block_to_stack(Block::ParentBlock(block));
    }

    fn handle_delimited_leaf_block(&mut self, asg: &mut Asg, block: LeafBlock) {
        if let Some(open_block) = self.block_stack.last_mut() {
            match open_block {
                Block::LeafBlock(leaf) => {
                    if leaf.name == block.name {
                        self.close_last_open_block(asg)
                    } else {
                        self.add_to_block_stack_or_graph(asg, Block::LeafBlock(block))
                    }
                }
                _ => self.push_block_to_stack(Block::LeafBlock(block)),
            }
        }
    }

    fn close_last_open_block(&mut self, asg: &mut Asg) {
        let Some(block) = self.block_stack.pop() else {
            panic!("Unexpected close last block call!")
        };
        self.add_to_block_stack_or_graph(asg, block)
    }

    fn push_block_to_stack(&mut self, block: Block) {
        // we only want to push on continue if we're not in an open delimited block (which will
        // close itself, emptying the open_delimited_block_lines)
        if self.in_block_continuation && self.open_delimited_block_lines.is_empty() {
            let Some(last_block) = self.block_stack.last_mut() else {
                panic!(
                    "Line {}: Invalid block continuation: no previous block",
                    block.line()
                )
            };
            last_block.push_block(block);
            self.in_block_continuation = false;
        } else {
            self.block_stack.push(block)
        }
    }

    fn add_text_to_last_inline(&mut self, token: Token) {
        let inline_literal = Inline::InlineLiteral(InlineLiteral::new_text_from_token(&token));
        if let Some(last_inline) = self.inline_stack.back_mut() {
            match last_inline {
                Inline::InlineSpan(span) => {
                    if self.in_inline_span {
                        span.add_inline(inline_literal);
                    } else {
                        self.inline_stack.push_back(inline_literal)
                    }
                }
                Inline::InlineLiteral(prior_literal) => prior_literal.add_text_from_token(&token),
                Inline::InlineRef(inline_ref) => {
                    if !self.close_parent_after_push {
                        inline_ref.inlines.push(inline_literal)
                    } else {
                        self.inline_stack.push_back(inline_literal)
                    }
                }
            }
        } else {
            self.inline_stack.push_back(inline_literal)
        }
    }

    fn add_inlines_to_block_stack(&mut self) {
        // guard
        if self.inline_stack.is_empty() {
            return;
        }

        if self.in_inline_span {
            // TK HANDLE DANGLING ITALICS
            todo!()
        }

        if let Some(last_block) = self.block_stack.last_mut() {
            if last_block.takes_inlines() && !self.in_block_line && !self.force_new_block {
                while !self.inline_stack.is_empty() {
                    let Some(inline) = self.inline_stack.pop_front() else {
                        panic!("Error getting inline from open queue");
                    };
                    last_block.push_inline(inline);
                }
                return;
            }
        }
        // create a new para from the locations of the first span (subsequent locations are
        // consolidated later)
        let mut para_locations: Vec<Location> = Vec::new();
        if let Some(first_inline) = self.inline_stack.front() {
            para_locations = first_inline.locations().clone();
        }
        let mut para_block = Block::LeafBlock(LeafBlock::new(
            crate::blocks::LeafBlockName::Paragraph,
            crate::blocks::LeafBlockForm::Paragraph,
            None,
            para_locations,
            vec![],
        ));
        while !self.inline_stack.is_empty() {
            if let Some(inline) = self.inline_stack.pop_front() {
                para_block.push_inline(inline)
            }
        }
        if self.in_block_continuation {
            let Some(last_block) = self.block_stack.last_mut() else {
                panic!(
                    "Line {}: Invalid block continuation: no previous block",
                    para_block.line()
                )
            };
            last_block.push_block(para_block);
        } else {
            self.push_block_to_stack(para_block)
        }
    }

    fn add_last_list_item_to_list(&mut self) {
        // clear out any forced new blocks
        self.force_new_block = false;
        // add the inlines to the list item
        self.add_inlines_to_block_stack();
        // then add it to the list
        let last_item = self.block_stack.pop().unwrap();
        // if the last thing is a list item, add it to the list
        if matches!(last_item, Block::ListItem(_)) {
            if let Some(list) = self.block_stack.last_mut() {
                list.push_block(last_item)
            }
        } else {
            // otherwise return the list to the open block stack, and create a new unordered
            // list item
            self.push_block_to_stack(last_item);
        }
    }

    fn add_to_block_stack_or_graph(&mut self, asg: &mut Asg, block: Block) {
        if let Some(last_block) = self.block_stack.last_mut() {
            if last_block.can_be_parent() {
                last_block.push_block(block);
                return;
            }
        }
        asg.push_block(block)
    }

    fn add_last_to_block_stack_or_graph(&mut self, asg: &mut Asg) {
        if let Some(last_block) = self.block_stack.pop() {
            if let Some(prior_block) = self.block_stack.last_mut() {
                if prior_block.can_be_parent() {
                    prior_block.push_block(last_block);
                    return;
                }
            }
            asg.push_block(last_block)
        } else {
            panic!("Tried to add last block when block stack was empty.")
        }
    }

    fn add_last_block_to_graph(&mut self, asg: &mut Asg) {
        // consolidate any list items
        if let Some(block) = self.block_stack.pop() {
            if let Some(next_last_block) = self.block_stack.last_mut() {
                if matches!(block, Block::ListItem(_)) {
                    // sanity check
                    if matches!(next_last_block, Block::List(_)) {
                        next_last_block.push_block(block);
                    } else {
                        //panic!("Dangling list item: missing parent list: {}", block.line())
                    }
                } else if next_last_block.is_section() {
                    next_last_block.push_block(block);
                    return;
                } else {
                    asg.push_block(block)
                }
            } else {
                asg.push_block(block)
            }
        }
    }
}
