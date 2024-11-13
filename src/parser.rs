use core::panic;
use std::collections::{HashMap, VecDeque};

use crate::{
    asg::Asg,
    blocks::{Block, BlockMacro, Break, LeafBlock, ParentBlock, Section},
    inlines::{Inline, InlineLiteral, InlineRef, InlineSpan, LineBreak},
    lists::{DList, DListItem, List, ListItem, ListVariant},
    metadata::{AttributeType, ElementMetadata},
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
    /// holding ground for a block title, to be applied to the subsequent block
    block_title: Option<Vec<Inline>>,
    /// holding ground for block metadata, to be applied to the subsequent block
    metadata: Option<ElementMetadata>,
    /// counts in/out delimited blocks by line reference; allows us to warn/error if they are
    /// unclosed at the end of the document
    open_delimited_block_lines: Vec<usize>,
    /// appends text to block or inline regardless of markup, token, etc. (will need to change
    /// if/when we handle code callouts)
    open_parse_after_as_text_type: Option<TokenType>,
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
    /// Temporarily preserves newline characters as separate inline literal tokens (where ambiguous
    /// blocks, i.e., DListItems, may require splitting the inline_stack on the newline)
    preserve_newline_text: bool,
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
            block_title: None,
            metadata: None,
            open_delimited_block_lines: vec![],
            in_document_header: false,
            in_block_line: false,
            in_inline_span: false,
            in_block_continuation: false,
            preserve_newline_text: false,
            open_parse_after_as_text_type: None,
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

        if let Some(token_type) = self.open_parse_after_as_text_type {
            match token_type {
                TokenType::QuoteVerseBlock => {
                    if token.token_type() == TokenType::QuoteVerseBlock || token.is_inline() {
                        self.open_parse_after_as_text_type = Some(token_type);
                    } else {
                        self.parse_text(token);
                        return;
                    }
                }
                TokenType::PassthroughInlineMacro => {
                    if [
                        TokenType::PassthroughInlineMacro,
                        TokenType::InlineMacroClose,
                    ]
                    .contains(&token.token_type())
                    {
                        self.open_parse_after_as_text_type = Some(token_type)
                    } else {
                        self.parse_text(token);
                        return;
                    }
                }
                TokenType::SourceBlock | TokenType::PassthroughBlock => {
                    if token.token_type() != token_type {
                        self.parse_text(token);
                        return;
                    }
                }
                _ => self.open_parse_after_as_text_type = Some(token_type),
            }
        }

        match token.token_type() {
            // document header, headings and section parsing
            TokenType::Heading1 => self.parse_level_0_heading(token, asg),
            TokenType::Heading2 => self.parse_section_headings(token, asg, 1),
            TokenType::Heading3 => self.parse_section_headings(token, asg, 2),
            TokenType::Heading4 => self.parse_section_headings(token, asg, 3),
            TokenType::Heading5 => self.parse_section_headings(token, asg, 4),

            // document attributes
            TokenType::Attribute => self.parse_attribute(token),

            // block titles, metadata, etc.
            TokenType::BlockLabel => {
                // open the block title
                self.block_title = Some(Vec::new());
                self.in_block_line = true;
                // clear out any dangling newlines
                self.dangling_newline = None;
            }
            TokenType::BlockAnchor => self.parse_block_anchor_attributes(token),
            TokenType::ElementAttributes => self.parse_block_element_attributes(token),

            // inline metadata
            TokenType::InlineStyle => self.parse_inline_element_attributes(token),

            // inlines
            TokenType::NewLineChar => self.parse_new_line_char(token, asg),
            TokenType::Text => self.parse_text(token),
            TokenType::Strong
            | TokenType::Mark
            | TokenType::Monospace
            | TokenType::Emphasis
            | TokenType::Superscript
            | TokenType::Subscript
            | TokenType::UnconstrainedStrong
            | TokenType::UnconstrainedMark
            | TokenType::UnconstrainedMonospace
            | TokenType::UnconstrainedEmphasis => self.parse_inline_span(Inline::InlineSpan(
                InlineSpan::inline_span_from_token(token),
            )),

            // references
            TokenType::AttributeReference => self.parse_attribute_reference(token),
            TokenType::CrossReference => self.parse_cross_reference(token),

            // inline macros
            TokenType::FootnoteMacro => self.parse_footnote_macro(token),
            TokenType::LinkMacro => self.parse_link_macro(token),
            TokenType::InlineImageMacro => self.parse_inline_image_macro(token),
            TokenType::PassthroughInlineMacro => self.parse_passthrough_inline_macro(token),
            TokenType::InlineMacroClose => self.parse_inline_macro_close(token),

            // breaks NEED TESTS
            TokenType::PageBreak => self.parse_page_break(token, asg),
            TokenType::ThematicBreak => self.parse_thematic_break(token, asg),
            TokenType::LineContinuation => self
                .inline_stack
                .push_back(Inline::InlineBreak(LineBreak::new_from_token(token))),

            // delimited blocks
            TokenType::SidebarBlock | TokenType::OpenBlock | TokenType::ExampleBlock => {
                self.parse_delimited_parent_block(token)
            }

            TokenType::QuoteVerseBlock => {
                // check if it's verse
                if let Some(metadata) = &self.metadata {
                    if metadata.declared_type == Some(AttributeType::Verse) {
                        self.parse_delimited_leaf_block(token);
                        return;
                    }
                } else if self.open_parse_after_as_text_type.is_some() {
                    self.parse_delimited_leaf_block(token);
                    return;
                }

                self.parse_delimited_parent_block(token);
            }

            // the following should probably be consumed into the above
            TokenType::PassthroughBlock => self.parse_delimited_leaf_block(token),
            TokenType::SourceBlock => self.parse_delimited_leaf_block(token),

            // block macros
            TokenType::BlockImageMacro => self.parse_block_image(token, asg),

            // lists
            TokenType::UnorderedListItem => self.parse_unordered_list_item(token),
            TokenType::OrderedListItem => self.parse_ordered_list_item(token),
            TokenType::DescriptionListMarker => self.parse_description_list_term(token),

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
            TokenType::CommentBlock => {
                // We treat a CommentBlock like a delimited LeafBlock, but throw away the result if
                // we've got a match
                if let Some(open_type) = self.open_parse_after_as_text_type {
                    if open_type == TokenType::CommentBlock {
                        self.inline_stack.clear();
                        self.block_stack.pop();
                        self.force_new_block = true;
                        self.open_parse_after_as_text_type = None;
                    }
                } else {
                    self.parse_delimited_leaf_block(token)
                }
            }

            _ => {
                // self check
                println!("\"{:?}\" not implemented", token.token_type());
                todo!()
            }
        }
    }

    fn parse_attribute(&mut self, token: Token) {
        let binding = token.text();
        let mut attr_components: Vec<&str> = binding.split_terminator(':').collect();
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

    fn parse_block_anchor_attributes(&mut self, token: Token) {
        self.add_metadata_from_token(token);
        self.force_new_block = true;
    }

    /// parses element attribute lists into self.block_metadata, which then is applied later
    fn parse_block_element_attributes(&mut self, token: Token) {
        self.add_metadata_from_token(token);
        self.force_new_block = true;
    }
    fn parse_inline_element_attributes(&mut self, token: Token) {
        self.metadata = Some(ElementMetadata::new_inline_meta_from_token(token));
        self.force_new_block = true;
    }

    /// New line characters are arguably the most significant token "signal" we can get, and as
    /// such the parse function is a little complicated.
    fn parse_new_line_char(&mut self, token: Token, asg: &mut Asg) {
        // newline exits a title, TK line continuation
        self.in_block_line = false;

        // if we had a block title, let's clean that out
        if let Some(ref mut title_stack) = self.block_title {
            while !self.inline_stack.is_empty() {
                let Some(inline) = self.inline_stack.pop_front() else {
                    panic!("Error getting inline from open queue");
                };
                title_stack.push(inline);
            }
        }

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
                // consolidate any dangling list items
                if let Some(Block::ListItem(_)) = self.block_stack.last() {
                    self.add_last_list_item_to_list();
                } else if let Some(Block::DListItem(_)) = self.block_stack.last() {
                    self.add_last_list_item_to_list();
                }
                // clear out any inlines
                self.in_inline_span = false;
                self.add_inlines_to_block_stack();
                // and then force a new block hereafter
                self.force_new_block = true;
                if let Some(last_block) = self.block_stack.pop() {
                    // check for dangling list items
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
        } else if self.in_block_continuation || self.last_token_type.clears_newline_after() {
            // don't add a newline ahead of text in these cases
            self.dangling_newline = None;
        } else {
            self.dangling_newline = Some(token);
        }
    }

    fn parse_thematic_break(&mut self, token: Token, asg: &mut Asg) {
        self.add_to_block_stack_or_graph(
            asg,
            Block::Break(Break::new(
                crate::blocks::BreakVariant::Thematic,
                token.locations(),
            )),
        )
    }

    fn parse_page_break(&mut self, token: Token, asg: &mut Asg) {
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

    /// Gathers preceding inlines into the "terms" attribute on DListItem, then adds what follows
    /// as you would for a normal list
    fn parse_description_list_term(&mut self, token: Token) {
        // create the list item
        let mut dlist_item = DListItem::new_from_token(token);

        // check for splits
        if let Some(newline_idx) = self
            .inline_stack
            .iter()
            .position(|inline| inline.is_newline())
        {
            // remove the inlines that ought to constitute the next term
            let mut next_terms: VecDeque<Inline> = self.inline_stack.drain(newline_idx..).collect();
            // remove the newline, since we don't care about that anymore
            next_terms.pop_front();
            // add the other inlines
            self.add_inlines_to_block_stack();
            // then add the next terms back
            self.inline_stack.append(&mut next_terms);
        }

        // collect the inlines
        while !self.inline_stack.is_empty() {
            let Some(inline) = self.inline_stack.pop_front() else {
                panic!("Error getting inline from open queue");
            };
            dlist_item.push_term(inline);
        }
        if self.block_stack.last().is_some()
            && self.block_stack.last().unwrap().is_definition_list_item()
        {
            self.add_last_list_item_to_list()
        } else {
            // we need to create the list first
            self.push_block_to_stack(Block::DList(DList::new(dlist_item.locations().clone())));
        }
        // either way, add the new list item
        self.push_block_to_stack(Block::DListItem(dlist_item));
        // preserve newlines for now
        self.preserve_newline_text = true;
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
        if token.first_location() == (Location { line: 1, col: 1 }) {
            self.in_document_header = true
        }
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
        // clear any dangling newlines, since we don't want these added to the title
        self.dangling_newline = None;
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

    /// Generic parser for inline spans that close themselves
    fn parse_inline_span(&mut self, mut inline: Inline) {
        if self.in_document_header && self.in_block_line {
            if let Some(last_inline) = self.document_header.title.last_mut() {
                if inline == *last_inline {
                    last_inline.reconcile_locations(inline.locations());
                    self.in_inline_span = false;
                    return;
                }
            }
            if self.metadata.is_some() {
                inline.add_metadata(self.metadata.as_ref().unwrap().clone());
                self.metadata = None;
            }
            self.document_header.title.push(inline);
        } else {
            if let Some(last_inline) = self.inline_stack.back_mut() {
                if inline == *last_inline {
                    last_inline.reconcile_locations(inline.locations());
                    last_inline.close();
                    self.in_inline_span = false;
                    return;
                }
            }
            // handle newline tokens prior to constrained spans
            if let Some(newline_token) = self.dangling_newline.clone() {
                self.add_text_to_last_inline(newline_token);
                self.dangling_newline = None;
            }
            if self.metadata.is_some() {
                inline.add_metadata(self.metadata.as_ref().unwrap().clone());
                self.metadata = None;
            }
            self.inline_stack.push_back(inline)
        }
        self.in_inline_span = true;
    }

    //fn parse_superscript(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_subscript(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_highlighted(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_link_macro(&mut self, token: Token) {
        self.inline_stack
            .push_back(Inline::InlineRef(InlineRef::new_link_from_token(token)))
    }

    fn parse_block_image(&mut self, token: Token, asg: &mut Asg) {
        let mut image_block = BlockMacro::new_image_from_token(token);
        if let Some(metadata) = &self.metadata {
            // TODO see if there is a cleaner way to manage the borrowing here.
            image_block = image_block.add_metadata(metadata);
            self.metadata = None;
        }
        self.add_to_block_stack_or_graph(asg, Block::BlockMacro(image_block));
    }

    fn parse_inline_image_macro(&mut self, token: Token) {
        self.inline_stack
            .push_back(Inline::InlineRef(InlineRef::new_inline_image_from_token(
                token,
            )));
        self.close_parent_after_push = true;
    }

    fn parse_footnote_macro(&mut self, token: Token) {
        self.inline_stack
            .push_back(Inline::InlineSpan(InlineSpan::inline_span_from_token(
                token,
            )));
        self.in_inline_span = true;
    }

    fn parse_passthrough_inline_macro(&mut self, token: Token) {
        self.open_parse_after_as_text_type = Some(token.token_type())
    }

    fn parse_inline_macro_close(&mut self, token: Token) {
        if let Some(TokenType::PassthroughInlineMacro) = self.open_parse_after_as_text_type {
            self.open_parse_after_as_text_type = None
        } else if let Some(inline_macro_idx) = self
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
            // and that the inline span is ended
            self.in_inline_span = false;
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

    fn parse_cross_reference(&mut self, token: Token) {
        self.inline_stack
            .push_back(Inline::InlineRef(InlineRef::new_xref_from_token(token)));
        self.close_parent_after_push = true;
    }

    fn parse_text(&mut self, token: Token) {
        if self.in_document_header && self.in_block_line {
            if let Some(inline) = self.document_header.title.last_mut() {
                match inline {
                    Inline::InlineLiteral(lit) => lit.add_text_from_token(&token),
                    Inline::InlineSpan(span) => {
                        let inline_lit =
                            Inline::InlineLiteral(InlineLiteral::new_text_from_token(&token));
                        if self.in_inline_span {
                            span.add_inline(inline_lit)
                        } else {
                            self.document_header.title.push(inline_lit)
                        }
                    }
                    Inline::InlineRef(_) => {
                        panic!("Inline references are not allowed in document titles")
                    }
                    Inline::InlineBreak(_) => {
                        panic!("Line breaks (+) are not allowed in document titles")
                    }
                }
            } else {
                self.document_header.title.push(Inline::InlineLiteral(
                    InlineLiteral::new_text_from_token(&token),
                ));
            }
        } else {
            if let Some(newline_token) = self.dangling_newline.clone() {
                if self.preserve_newline_text {
                    // add the newline as such
                    self.inline_stack.push_back(Inline::InlineLiteral(
                        InlineLiteral::new_text_from_token(&newline_token),
                    ));
                    // clear the newline
                    self.dangling_newline = None;
                    // add the new text separately
                    self.inline_stack.push_back(Inline::InlineLiteral(
                        InlineLiteral::new_text_from_token(&token),
                    ));
                    return;
                } else {
                    self.add_text_to_last_inline(newline_token);
                    self.dangling_newline = None;
                }
            }
            self.add_text_to_last_inline(token)
        }
    }

    /// TODO some magic about the inlines, etc.
    fn parse_delimited_leaf_block(&mut self, token: Token) {
        if self.open_parse_after_as_text_type.is_some() {
            let Some(open_leaf) = self.block_stack.last_mut() else {
                panic!("Missing last block on the stack")
            };
            open_leaf.add_locations(token.locations().clone());
            self.open_parse_after_as_text_type = None;
        } else {
            self.open_parse_after_as_text_type = Some(token.token_type());
            let block = LeafBlock::new_from_token(token);
            self.push_block_to_stack(Block::LeafBlock(block));
            // note that we're to just add
            self.force_new_block = false;
        }
    }

    // TODO: handle [NOTE]\n==== cases (i.e., some block metadata check)
    fn parse_delimited_parent_block(&mut self, token: Token) {
        let delimiter_line = token.first_location().line;
        let mut block = ParentBlock::new_from_token(token);
        // clear the dangling newline
        self.dangling_newline = None;

        if self.block_title.is_some() {
            block.title = self.block_title.as_ref().unwrap().clone();
            self.block_title = None;
        }

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
                // close any dangling inlines BEFORE opening the delimited block lines
                self.add_inlines_to_block_stack();
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
                // collect subsequent blocks to be added to the parent block
                let mut blocks_to_add =
                    VecDeque::from_iter(self.block_stack.drain(parent_block_idx..));
                //
                let mut delimited_block = Block::ParentBlock(matched);
                while !blocks_to_add.is_empty() {
                    delimited_block.push_block(blocks_to_add.pop_front().unwrap())
                }
                self.push_block_to_stack(delimited_block);
                // close any continuations
                self.in_block_continuation = false;
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

    fn push_block_to_stack(&mut self, mut block: Block) {
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
            if self.metadata.is_some() {
                block.add_metadata(self.metadata.as_ref().unwrap().clone());
                self.metadata = None;
            }
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
                        inline_ref.add_text_from_token(token)
                    } else {
                        self.inline_stack.push_back(inline_literal)
                    }
                }
                Inline::InlineBreak(_) => {
                    // can't add to the back, so just add the literal
                    self.inline_stack.push_back(inline_literal)
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

        // dangling inlines
        if self.in_inline_span {
            // look for the last span in the stack
            let Some(open_span_idx) = self
                .inline_stack
                .iter()
                .rposition(|inline| inline.is_open())
            else {
                println!("This inline opening has not been handled!");
                panic!()
            };
            let mut open_span = self.inline_stack.remove(open_span_idx).unwrap();
            let open_span_literal = open_span.produce_literal_from_self();
            // put the literal char into the stack as a literal...
            let mut children = open_span.extract_child_inlines();
            // ... by adding it to the next inline
            if let Some(inline) = children.front_mut() {
                match inline {
                    Inline::InlineLiteral(literal) => {
                        literal.prepend_to_value(open_span_literal, open_span.locations())
                    }
                    _ => todo!(),
                }
                // put any appended inlines into the stack at the relevant position
                for child_inline in children {
                    self.inline_stack.insert(open_span_idx, child_inline)
                }
            } else {
                // ... or if there are no children, to the back
                // this is hacky, but it is cleaner compared to the rest of the code just to
                // create a token and reuse the existing function
                let (line, startcol, endcol) =
                    Location::destructure_inline_locations(open_span.locations());
                let reconstituted_token = Token {
                    token_type: TokenType::Text,
                    lexeme: open_span_literal.clone(),
                    literal: Some(open_span_literal),
                    line,
                    startcol,
                    endcol,
                };
                self.add_text_to_last_inline(reconstituted_token)
            }
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
        if self.in_block_continuation && self.open_delimited_block_lines.is_empty() {
            let Some(last_block) = self.block_stack.last_mut() else {
                panic!(
                    "Line {}: Invalid block continuation: no previous block",
                    para_block.line()
                )
            };
            last_block.push_block(para_block);
            return;
        }

        if self.metadata.is_some() {
            // check to see if we need to apply metadata to the para block
            let line_above = para_block.locations().first().unwrap().line + 1;
            if self.open_delimited_block_lines.last() == Some(&line_above)
                || self.open_delimited_block_lines.is_empty()
            {
                let Some(ref metadata) = self.metadata else {
                    panic!()
                };
                if metadata.declared_type == Some(AttributeType::Quote) {
                    let mut quote_block = Block::ParentBlock(ParentBlock::new(
                        crate::blocks::ParentBlockName::Quote,
                        None,
                        "".to_string(),
                        vec![],
                        vec![],
                    ));
                    quote_block.push_block(para_block);
                    self.push_block_to_stack(quote_block);
                    return;
                }
            }
        }
        self.push_block_to_stack(para_block)
    }

    /// Adds data to an existing ElementMetadata object, or creates one
    fn add_metadata_from_token(&mut self, token: Token) {
        println!("{:?}, {:?}", self.metadata, token);
        match self.metadata {
            Some(ref mut metadata) => metadata.add_metadata_from_token(token),
            None => {
                self.metadata = Some(ElementMetadata::new_block_meta_from_token(token));
            }
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
        } else if matches!(last_item, Block::DListItem(_)) {
            if let Some(list) = self.block_stack.last_mut() {
                list.push_block(last_item)
            }
        } else {
            // otherwise return the list to the open block stack, and create a new unordered
            // list item
            self.push_block_to_stack(last_item);
        }
    }

    fn add_to_block_stack_or_graph(&mut self, asg: &mut Asg, mut block: Block) {
        if let Some(last_block) = self.block_stack.last_mut() {
            if last_block.can_be_parent() {
                last_block.push_block(block);
                return;
            }
        }
        if self.metadata.is_some() {
            block.add_metadata(self.metadata.as_ref().unwrap().clone());
            self.metadata = None;
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
        if let Some(mut block) = self.block_stack.pop() {
            if self.metadata.is_some() {
                block.add_metadata(self.metadata.as_ref().unwrap().clone());
                self.metadata = None;
            }
            if let Some(next_last_block) = self.block_stack.last_mut() {
                if matches!(block, Block::ListItem(_)) {
                    // sanity check
                    if matches!(next_last_block, Block::List(_)) {
                        next_last_block.push_block(block);
                    } else {
                        //panic!("Dangling list item: missing parent list: {}", block.line())
                    }
                } else if matches!(block, Block::DListItem(_)) {
                    next_last_block.push_block(block);
                } else if next_last_block.can_be_parent() {
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
