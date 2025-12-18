//! Parses a stream of tokens into an [`Asg`] (Abstract Syntax Graph)
use std::{
    collections::{HashMap, VecDeque},
    env,
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use log::{error, warn};

use crate::scanner::tokens::{Token, TokenType};
use crate::{
    errors::{ParserError, ScannerError},
    scanner::Scanner,
};
use crate::{
    graph::{
        asg::Asg,
        blocks::{Block, BlockMacro, Break, LeafBlock, ParentBlock, Section, TableCell},
        inlines::{Inline, InlineLiteral, InlineLiteralName, InlineRef, InlineSpan, LineBreak},
        lists::{DList, DListItem, List, ListItem, ListVariant},
        metadata::{AttributeType, ElementMetadata},
        nodes::{Header, Location},
    },
    utils::{extract_page_ranges, target_and_attrs_from_token},
};

/// Parses a stream of tokens into an [`Asg`] (Abstract Syntax Graph), returning the graph once all
/// tokens have been parsed.
#[derive(Debug)]
pub struct Parser {
    /// Where the parsing "starts," i.e., the adoc file passed to the script
    origin_directory: PathBuf,
    /// allows for "what just happened" matching
    last_token_type: TokenType,
    /// optional document header
    document_header: Option<Header>,
    /// document-level attributes, used for replacements, etc.
    document_attributes: HashMap<String, String>,
    /// holding ground for graph blocks until it's time to push to the main graph
    block_stack: Vec<Block>,
    /// holding ground for inline elements until it's time to push to the relevant block
    inline_stack: VecDeque<Inline>,
    /// holding ground for includes file names; if inside an include push to stack, popping off
    /// once the file's tokens have been accommodated (this allows for simpler nesting)
    file_stack: Vec<String>,
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
    /// tracks the document heading level offset
    level_offset: i8,
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
    /// Testing helper
    pub resolve_targets: bool,
}

impl Default for Parser {
    fn default() -> Self {
        // defaults assuming stdin
        match env::current_dir() {
            Ok(dir) => Self::new(dir),
            Err(e) => {
                error!("Unexpeced error: {e}");
                std::process::exit(1)
            }
        }
    }
}

impl Parser {
    /// Initializes the parser with a "origin", which is either the current directory (in the case
    /// of STDIN (`-`)) or the parent of the provided asciidoc file; this origin is required for
    /// correct `include::` directive resolution.
    pub fn new(origin: PathBuf) -> Self {
        let origin_directory = origin
            .parent()
            .unwrap_or(&env::current_dir().unwrap())
            .to_path_buf();
        Parser {
            origin_directory,
            last_token_type: TokenType::Eof,
            document_header: None,
            document_attributes: HashMap::new(),
            block_stack: vec![],
            inline_stack: VecDeque::new(),
            file_stack: vec![],
            block_title: None,
            metadata: None,
            open_delimited_block_lines: vec![],
            level_offset: 0,
            in_block_line: false,
            in_inline_span: false,
            in_block_continuation: false,
            preserve_newline_text: false,
            open_parse_after_as_text_type: None,
            force_new_block: false,
            close_parent_after_push: false,
            dangling_newline: None,
            resolve_targets: true,
        }
    }

    /// Similar to Parser::new() except returns a parser that does not resolve non-include
    /// directive targets (useful for testing, or in cases where you don't care if a given target
    /// resource exists or not)
    pub fn new_no_target_resolution(origin: PathBuf) -> Self {
        let mut test_parser = Self::new(origin);
        test_parser.resolve_targets = false;
        test_parser
    }

    /// Parses the stream of tokens provided by the [`Scanner`].
    pub fn parse<I>(&mut self, tokens: I) -> Result<Asg, ParserError>
    where
        I: Iterator<Item = Result<Token, ScannerError>>,
    {
        let mut asg = Asg::new();
        for result in tokens {
            match result {
                Ok(token) => {
                    let token_type = token.token_type();
                    self.token_into(token, &mut asg)?;

                    self.last_token_type = token_type;
                }
                Err(e) => return Err(ParserError::Scanner(e)),
            }
        }

        // add any dangling inlines
        self.add_inlines_to_block_stack()?;
        // add any dangling blocks (most often sections)
        while !self.block_stack.is_empty() {
            self.add_last_block_to_graph(&mut asg)?;
        }
        // cleanup the final tree locations and xrefs
        asg.consolidate();
        Ok(asg)
    }

    fn token_into(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        // if we are not starting with a document-heading acceptable token, get out
        if self.document_header.is_some() && !token.can_be_in_document_header() {
            self.check_and_move_header(asg)?;
        }

        if let Some(token_type) = self.open_parse_after_as_text_type {
            match token_type {
                TokenType::QuoteVerseBlock => {
                    if token.token_type() == TokenType::QuoteVerseBlock || token.is_inline() {
                        self.open_parse_after_as_text_type = Some(token_type);
                    } else {
                        self.parse_text(token)?;
                        return Ok(());
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
                        self.pass_text_through(token)?;
                        return Ok(());
                    }
                }
                TokenType::PassthroughBlock | TokenType::LiteralBlock | TokenType::CommentBlock => {
                    if token.token_type() != token_type {
                        self.pass_text_through(token)?;
                        return Ok(());
                    }
                }
                TokenType::SourceBlock => {
                    // allow callouts and includes
                    if ![token_type, TokenType::CodeCallout, TokenType::Include]
                        .contains(&token.token_type())
                    {
                        self.pass_text_through(token)?;
                        return Ok(());
                    }
                }
                _ => self.open_parse_after_as_text_type = Some(token_type),
            }
        }

        match token.token_type() {
            // document header, headings and section parsing
            TokenType::Heading1 => self.parse_title(token, asg),
            TokenType::Heading2 => self.parse_section_headings(token, asg),
            TokenType::Heading3 => self.parse_section_headings(token, asg),
            TokenType::Heading4 => self.parse_section_headings(token, asg),
            TokenType::Heading5 => self.parse_section_headings(token, asg),

            // document attributes
            TokenType::Attribute => self.parse_attribute(token),

            // block titles, metadata, etc.
            TokenType::BlockLabel => {
                // open the block title
                self.block_title = Some(Vec::new());
                self.in_block_line = true;
                // clear out any dangling newlines
                self.dangling_newline = None;
                Ok(())
            }
            TokenType::BlockAnchor => self.parse_block_anchor_attributes(token),
            TokenType::ElementAttributes => self.parse_block_element_attributes(token),

            // inline metadata
            TokenType::InlineStyle => self.parse_inline_element_attributes(token),

            // inlines
            TokenType::NewLineChar => self.parse_new_line_char(token, asg),
            TokenType::Text => self.parse_text(token),
            TokenType::CharRef => self.parse_charref(token),
            TokenType::Strong
            | TokenType::Mark
            | TokenType::Monospace
            | TokenType::Literal
            | TokenType::Emphasis
            | TokenType::Superscript
            | TokenType::Subscript
            | TokenType::UnconstrainedStrong
            | TokenType::UnconstrainedMark
            | TokenType::UnconstrainedMonospace
            | TokenType::UnconstrainedLiteral
            | TokenType::UnconstrainedEmphasis => self.parse_inline_span(Inline::InlineSpan(
                InlineSpan::inline_span_from_token(token),
            )),

            // replacements -- done on token.text()
            TokenType::OpenDoubleQuote
            | TokenType::CloseDoubleQuote
            | TokenType::OpenSingleQuote
            | TokenType::CloseSingleQuote => self.parse_text(token),

            // references
            TokenType::AttributeReference => self.parse_attribute_reference(token),
            TokenType::CrossReference => self.parse_cross_reference(token),
            TokenType::Include => self.parse_include(token, asg),
            // we just check for the existence of these; we don't actually process them
            TokenType::StartTag | TokenType::EndTag => {Ok(())}

            // inline macros
            TokenType::FootnoteMacro => self.parse_footnote_macro(token),
            TokenType::LinkMacro => self.parse_link_macro(token),
            TokenType::Hyperlink => self.parse_hyperlink(token),
            TokenType::InlineImageMacro => self.parse_inline_image_macro(token),
            TokenType::PassthroughInlineMacro => self.parse_passthrough_inline_macro(token),
            TokenType::InlineMacroClose => self.parse_inline_macro_close(token),

            // breaks
            TokenType::PageBreak => self.parse_page_break(token, asg),
            TokenType::ThematicBreak => self.parse_thematic_break(token, asg),
            TokenType::LineContinuation => {self
                .inline_stack
                .push_back(Inline::InlineBreak(LineBreak::new_from_token(token)));Ok(())},
            // delimited blocks
            TokenType::SidebarBlock
            | TokenType::OpenBlock
            | TokenType::ExampleBlock
            | TokenType::Table => self.parse_delimited_parent_block(token),

            // table cells -- note that we just create the cells; up to the backend/template to handle the
            // column-making (for now, so we can just reuse ParentBlock)
            TokenType::TableCell => self.parse_table_cell(token, asg),

            TokenType::QuoteVerseBlock => {
                // check if it's verse
                if let Some(metadata) = &self.metadata {
                    if metadata.declared_type == Some(AttributeType::Verse) {
                        self.parse_delimited_leaf_block(token)?;
                        return Ok(());
                    }
                } else if self.open_parse_after_as_text_type.is_some() {
                    self.parse_delimited_leaf_block(token)?;
                    return Ok(());
                }

                self.parse_delimited_parent_block(token)?;
                Ok(())
            }

            // the following should probably be consumed into the above
            TokenType::PassthroughBlock | TokenType::LiteralBlock => {
                self.parse_delimited_leaf_block(token)
            }
            TokenType::SourceBlock => self.parse_delimited_leaf_block(token),
            TokenType::CodeCallout => self.parse_code_callout(token),

            // block macros
            TokenType::BlockImageMacro => self.parse_block_image(token, asg),

            // lists
            TokenType::UnorderedListItem => self.parse_unordered_list_item(token),
            TokenType::CodeCalloutListItem | // for now, match here (until we need to do more)
            TokenType::OrderedListItem => self.parse_ordered_list_item(token, asg),
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
                    Ok(())
                } else {
                    self.parse_delimited_leaf_block(token)
                }
            }

            TokenType::Eof => {
                if self.document_header.is_some() {
                    self.check_and_move_header(asg)?
                }
                Ok(())
            }
            TokenType::Email => todo!()
        }
    }

    fn parse_attribute(&mut self, token: Token) -> Result<(), ParserError> {
        let binding = token.text();
        let mut attr_components: Vec<&str> = binding.split_terminator(':').collect();
        attr_components.remove(0); // throw away initial "" in the list
        if attr_components.is_empty() {
            warn!("Empty attributes list at line: {}", token.line);
            return Ok(());
        }
        let key = attr_components.first().unwrap();
        // values should be trimmed
        let mut value = attr_components.last().unwrap().trim();
        if key == &value {
            value = ""
        }
        match *key {
            "leveloffset" => self.parse_level_offset(value),
            _ => {
                self.document_attributes
                    .insert(key.to_string(), value.to_string());
                Ok(())
            }
        }
    }

    fn parse_level_offset(&mut self, value: &str) -> Result<(), ParserError> {
        if let Ok(value) = value.parse::<usize>() {
            self.level_offset = value as i8;
            Ok(())
        } else {
            match value.parse::<i8>() {
                // absolute value
                Ok(value) => {
                    self.level_offset += value;
                    Ok(())
                }
                // relative value, or error
                Err(_) => Err(ParserError::InternalError(
                    "Error parsing level offset".to_string(),
                )),
            }
        }
    }

    fn parse_block_anchor_attributes(&mut self, token: Token) -> Result<(), ParserError> {
        self.add_metadata_from_token(token);
        self.force_new_block = true;
        Ok(())
    }

    /// parses element attribute lists into self.block_metadata, which then is applied later
    fn parse_block_element_attributes(&mut self, token: Token) -> Result<(), ParserError> {
        self.add_metadata_from_token(token);
        self.force_new_block = true;
        Ok(())
    }
    fn parse_inline_element_attributes(&mut self, token: Token) -> Result<(), ParserError> {
        self.metadata = Some(ElementMetadata::new_inline_meta_from_token(token));
        self.force_new_block = true;
        Ok(())
    }

    /// Handle document header
    fn check_and_move_header(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
        self.add_inlines_to_block_stack()?;
        if let Some(header) = &mut self.document_header {
            if !header.is_empty() {
                header.consolidate();
                asg.add_header(header.clone(), self.document_attributes.clone())
            }
        }

        self.document_header = None;
        Ok(())
    }

    /// New line characters are arguably the most significant token "signal" we can get, and as
    /// such the parse function is a little complicated.
    fn parse_new_line_char(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        // newline exits a title, TK line continuation
        self.in_block_line = false;

        // if there is a block title, add the inline stack to the title
        if let Some(ref mut title_stack) = self.block_title {
            while !self.inline_stack.is_empty() {
                let inline = self.inline_stack.pop_front().unwrap();
                title_stack.push(inline);
            }
        }

        if [TokenType::NewLineChar, TokenType::Eof].contains(&self.last_token_type) {
            // clear any dangling newline
            self.dangling_newline = None;
            if self.document_header.is_some() {
                self.check_and_move_header(asg)?;
            } else {
                // consolidate any dangling list items
                if let Some(Block::ListItem(_)) = self.block_stack.last() {
                    self.add_last_list_item_to_list()?;
                } else if let Some(Block::DListItem(_)) = self.block_stack.last() {
                    self.add_last_list_item_to_list()?;
                }
                // clear out any inlines
                self.in_inline_span = false;
                self.add_inlines_to_block_stack()?;
                // and then force a new block hereafter
                self.force_new_block = true;
                if let Some(last_block) = self.block_stack.pop() {
                    // check for dangling list items
                    if !last_block.is_section() && self.open_delimited_block_lines.is_empty() {
                        self.add_to_block_stack_or_graph(asg, last_block)?;
                        if self.close_parent_after_push && !self.block_stack.is_empty() {
                            self.add_last_to_block_stack_or_graph(asg)?;
                            self.close_parent_after_push = false;
                        }
                    } else {
                        self.push_block_to_stack(last_block)?
                    }
                } // if Some(last_block)
            }
        } else if self.in_block_continuation || self.last_token_type.clears_newline_after() {
            // don't add a newline ahead of text in these cases
            self.dangling_newline = None;
        } else {
            self.dangling_newline = Some(token);
        }
        Ok(())
    }

    fn parse_thematic_break(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        self.add_to_block_stack_or_graph(
            asg,
            Block::Break(Break::new(
                crate::graph::blocks::BreakVariant::Thematic,
                token.locations(),
            )),
        )
    }

    fn parse_page_break(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        self.add_to_block_stack_or_graph(
            asg,
            Block::Break(Break::new(
                crate::graph::blocks::BreakVariant::Page,
                token.locations(),
            )),
        )
    }

    // Comments
    fn parse_comment(&self) -> Result<(), ParserError> {
        // for now, do nothing
        Ok(())
    }

    fn parse_include(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        // placeholders
        let mut included_lines: Vec<i32> = vec![];
        let mut include_to_end: bool = false;
        let mut included_tags: Vec<String> = vec![];
        let mut asciidoc_include: bool = false;
        let mut current_tag: Option<String> = None;

        let (target, meta) = target_and_attrs_from_token(&token);
        // check for level offsets in the include
        if let Some(metadata) = meta {
            if let Some(value) = metadata.attributes.get("leveloffset") {
                match self.parse_level_offset(value) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
            if let Some(value) = metadata.attributes.get("lines") {
                included_lines = extract_page_ranges(value);
                if let Some(line) = included_lines.last() {
                    if *line == -1 {
                        include_to_end = true;
                        included_lines.pop();
                    }
                }
            }
            if let Some(value) = metadata.attributes.get("tag") {
                included_tags.push(value.clone());
            } // it can either be tag or tags, which is annoying!
            if let Some(value) = metadata.attributes.get("tags") {
                included_tags.extend(value.split(";").map(|tag| tag.to_string()));
            }
        }

        // calculate target, given that it's relative; if there is something on the stack, use
        // that, else use self.origin
        let resolved_target = self.resolve_target(token.line, &target)?;
        self.file_stack.push(target.clone());

        let current_block_stack_len = self.block_stack.len();

        // Are we including another asciidoc file that should be scanned and parsed?
        if matches!(
            target.split('.').next_back().unwrap_or(""),
            "adoc" | "asciidoc" | "txt" // note that per the spec we parse txt as asciidoc
        ) {
            asciidoc_include = true;
        }

        // now parse the file
        for result in Scanner::new_with_stack(&open_file(resolved_target), self.file_stack.clone())
        {
            match result {
                Ok(token) => {
                    // lines
                    if !included_lines.is_empty() {
                        if !included_lines.contains(&(token.line as i32)) {
                            continue;
                        } else if include_to_end
                            && &(token.line as i32) >= included_lines.last().unwrap()
                        {
                            included_lines.clear()
                        }
                    }
                    // tags
                    if !included_tags.is_empty() {
                        if token.token_type() == TokenType::StartTag {
                            if let Some(tag) = token.tag() {
                                if included_tags.contains(&tag) {
                                    current_tag = Some(tag);
                                    continue;
                                }
                            }
                        }
                        if token.token_type() == TokenType::EndTag {
                            if let Some(tag) = token.tag() {
                                if current_tag == Some(tag) {
                                    current_tag = None;
                                    continue;
                                }
                            }
                        }
                        if current_tag.is_none() {
                            continue;
                        }
                    }
                    if asciidoc_include {
                        let token_type = token.token_type();
                        self.token_into(token, asg)?;
                        self.last_token_type = token_type;
                    } else {
                        // allow EOFs to pass through; otherwise just parse as text
                        if matches!(token.token_type(), TokenType::Eof) {
                            self.last_token_type = TokenType::Eof;
                        } else {
                            self.last_token_type = TokenType::Text;
                        }
                        self.parse_text(token)?;
                    }
                }
                Err(e) => return Err(ParserError::Scanner(e)),
            }
        }

        // clean up inlines
        self.add_inlines_to_block_stack()?;

        // get the blocks stack back to where it was
        while self.block_stack.len() > current_block_stack_len {
            self.add_last_block_to_graph(asg)?
        }
        // ...then pop the file off the stack
        self.file_stack.pop();
        Ok(())
    }

    /// Gathers preceding inlines into the "terms" attribute on DListItem, then adds what follows
    /// as you would for a normal list
    fn parse_description_list_term(&mut self, token: Token) -> Result<(), ParserError> {
        // create the list item
        let mut dlist_item = DListItem::new_from_token(token);
        // ensure we're not forcing a new block, given that all sorts of things can follow
        self.force_new_block = false;

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
            self.add_inlines_to_block_stack()?;
            // then add the next terms back
            self.inline_stack.append(&mut next_terms);
        }
        // collect the inlines
        while !self.inline_stack.is_empty() {
            let inline = self.inline_stack.pop_front().unwrap();
            dlist_item.push_term(inline);
        }
        if self.block_stack.last().is_some()
            && self.block_stack.last().unwrap().is_definition_list_item()
        {
            self.add_last_list_item_to_list()?
        } else {
            // we need to create the list first
            self.push_block_to_stack(Block::DList(DList::new(dlist_item.locations().clone())))?;
        }
        // either way, add the new list item
        self.push_block_to_stack(Block::DListItem(dlist_item))?;
        // preserve newlines for now
        self.preserve_newline_text = true;
        Ok(())
    }

    fn parse_ordered_list_item(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        // clear any dangling newlines
        self.dangling_newline = None;
        let list_item = ListItem::new(token.lexeme.clone(), token.locations());
        // if there is an appropriate list, we push this onto the open_blocks so inlines can be
        // added
        if self.block_stack.last().is_some()
            && self.block_stack.last().unwrap().is_ordered_list_item()
        {
            self.add_last_list_item_to_list()?
        } else {
            // we need to create the list first
            let mut list = List::new(ListVariant::Ordered, token.locations().clone());
            if token.token_type() == TokenType::CodeCalloutListItem {
                // check to see if we ought to "close" the source block (almost always)
                // TODO source blocks should close themselves, I think.
                if let Some(block) = self.block_stack.last() {
                    if block.is_source_block() {
                        // we need to add this before we create the new list
                        self.add_last_to_block_stack_or_graph(asg)?;
                    }
                }
                list.metadata = Some(ElementMetadata::new_with_role("colist".to_string()));
            }
            self.push_block_to_stack(Block::List(list))?;
        }
        // either way, add the new list item
        self.push_block_to_stack(Block::ListItem(list_item))
    }
    //
    fn parse_unordered_list_item(&mut self, token: Token) -> Result<(), ParserError> {
        // clear any dangling newlines
        self.dangling_newline = None;
        let list_item = ListItem::new(token.lexeme.clone(), token.locations());
        // if there is an appropriate list, we push this onto the open_blocks so inlines can be
        // added
        if self.block_stack.last().is_some()
            && self.block_stack.last().unwrap().is_unordered_list_item()
        {
            self.add_last_list_item_to_list()?
        } else {
            // we need to create the list first
            self.push_block_to_stack(Block::List(List::new(
                ListVariant::Unordered,
                token.locations().clone(),
            )))?;
        }
        // either way, add the new list item
        self.push_block_to_stack(Block::ListItem(list_item))
    }

    //fn parse_block_label(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_title(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        // Check offsets
        if self.level_offset != 0 {
            return self.parse_section_headings(token, asg);
        }

        if token.first_location() == Location::default() {
            self.in_block_line = true;
            let mut header = Header::new();
            header.location.extend(token.locations());

            self.document_header = Some(header);
            Ok(())
        } else {
            Err(ParserError::TopLevelHeading(token.line))
        }
    }

    fn parse_section_headings(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        let level = self.get_heading_level(&token)?;

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
        // clear any dangling newlines, since we don't want these added to the title
        self.dangling_newline = None;
        // let us know that we want to add to the section title for a little bit
        self.force_new_block = false;
        Ok(())
    }

    /// Returns a valid heading level for a given token
    fn get_heading_level(&self, token: &Token) -> Result<usize, ParserError> {
        let level: i8 = match token.token_type() {
            TokenType::Heading1 => self.level_offset,
            TokenType::Heading2 => 1 + self.level_offset,
            TokenType::Heading3 => 2 + self.level_offset,
            TokenType::Heading4 => 3 + self.level_offset,
            TokenType::Heading5 => 4 + self.level_offset,
            _ => {
                return Err(ParserError::InternalError(
                    "Inavlid token given to parse_section_headings".to_string(),
                ));
            }
        };

        match level.try_into() {
            Ok(value) => {
                if !(1..=4).contains(&value) {
                    Err(ParserError::HeadingOffsetError(
                        token.line,
                        self.level_offset,
                    ))
                } else {
                    Ok(value)
                }
            }
            Err(_) => Err(ParserError::HeadingOffsetError(
                token.line,
                self.level_offset,
            )),
        }
    }

    fn parse_admonition_para_syntax(&mut self, token: Token) -> Result<(), ParserError> {
        self.block_stack
            .push(Block::ParentBlock(ParentBlock::new_from_token(token)?));
        self.close_parent_after_push = true;
        Ok(())
    }

    fn parse_block_continuation(&mut self, _token: Token) -> Result<(), ParserError> {
        self.add_inlines_to_block_stack()?;
        self.in_block_continuation = true;
        self.force_new_block = true;
        Ok(())
    }

    /// Generic parser for inline spans that close themselves
    fn parse_inline_span(&mut self, mut inline: Inline) -> Result<(), ParserError> {
        if let Some(last_inline) = self.inline_stack.back_mut() {
            if inline == *last_inline {
                last_inline.reconcile_locations(inline.locations());
                last_inline.close();
                self.in_inline_span = false;
                return Ok(());
            } else if let Inline::InlineSpan(last_span) = last_inline {
                if let Some(last_internal_inline) = last_span.inlines.last_mut() {
                    // okay so what needs to happen is that spans deconstruct themselves,
                    // I think
                    if inline == *last_internal_inline {
                        last_internal_inline.reconcile_locations(inline.locations());
                        last_internal_inline.close();
                    } else {
                        last_span.add_inline(inline);
                    }
                } else {
                    last_span.add_inline(inline);
                }
                return Ok(());
            }
        }
        // handle newline tokens prior to constrained spans
        if let Some(newline_token) = self.dangling_newline.clone() {
            self.add_text_to_last_inline(newline_token);
            self.dangling_newline = None;
        }
        // check for any other dangling spans (otherwise they disappear!)
        self.handle_dangling_spans();
        // add any metadata to new inline
        if self.metadata.is_some() {
            inline.add_metadata(self.metadata.as_ref().unwrap().clone());
            self.metadata = None;
        }
        self.inline_stack.push_back(inline);

        self.in_inline_span = true;
        Ok(())
    }

    fn parse_link_macro(&mut self, token: Token) -> Result<(), ParserError> {
        self.inline_stack
            .push_back(Inline::InlineRef(InlineRef::new_link_from_macro_token(
                token,
            )));
        Ok(())
    }

    fn parse_hyperlink(&mut self, token: Token) -> Result<(), ParserError> {
        self.inline_stack
            .push_back(Inline::InlineRef(InlineRef::new_link_from_token(token)));
        Ok(())
    }

    fn parse_block_image(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        let (target, metadata) = target_and_attrs_from_token(&token);
        self.check_target(token.line, &target)?;
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

    fn parse_inline_image_macro(&mut self, token: Token) -> Result<(), ParserError> {
        let (target, metadata) = target_and_attrs_from_token(&token);
        let _ = self.check_target(token.line, &target);
        self.inline_stack
            .push_back(Inline::InlineRef(InlineRef::new_inline_image(
                target,
                metadata,
                token.locations(),
            )));
        self.close_parent_after_push = true;
        Ok(())
    }

    fn parse_footnote_macro(&mut self, token: Token) -> Result<(), ParserError> {
        self.inline_stack
            .push_back(Inline::InlineSpan(InlineSpan::inline_span_from_token(
                token,
            )));
        self.in_inline_span = true;
        Ok(())
    }

    fn parse_passthrough_inline_macro(&mut self, token: Token) -> Result<(), ParserError> {
        self.open_parse_after_as_text_type = Some(token.token_type());
        Ok(())
    }

    fn parse_inline_macro_close(&mut self, token: Token) -> Result<(), ParserError> {
        if let Some(TokenType::PassthroughInlineMacro) = self.open_parse_after_as_text_type {
            self.open_parse_after_as_text_type = None
        } else if let Some(inline_macro_idx) = self
            .inline_stack
            .iter()
            .rposition(|inline| inline.is_macro())
        {
            // consolidate into the inline macro
            let mut inline_macro = self.inline_stack.remove(inline_macro_idx).unwrap();
            while self.inline_stack.len() > inline_macro_idx {
                let subsequent_inline = self.inline_stack.remove(inline_macro_idx).unwrap();
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
        Ok(())
    }

    /// attribute references become literals, but we need to replace them with the appropriate
    /// values from the document header first
    fn parse_attribute_reference(&mut self, mut token: Token) -> Result<(), ParserError> {
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
            warn!("Missing document attribute: {}", attribute_target);
        }
        // then add it as literal text
        self.parse_text(token)?;
        Ok(())
    }

    fn parse_cross_reference(&mut self, token: Token) -> Result<(), ParserError> {
        self.inline_stack
            .push_back(Inline::InlineRef(InlineRef::new_xref_from_token(token)));
        self.close_parent_after_push = true;
        Ok(())
    }

    fn parse_code_callout(&mut self, token: Token) -> Result<(), ParserError> {
        if let Some(value) = self.document_attributes.get("icons") {
            if value == "true" {
                if let Some(_last_inline) = self.inline_stack.back_mut() {
                    // handle deleting comment markup, IF we're handling icons
                    todo!();
                }
            }
        }
        self.inline_stack
            .push_back(Inline::InlineSpan(InlineSpan::inline_span_from_token(
                token,
            )));
        Ok(())
    }

    fn parse_text(&mut self, token: Token) -> Result<(), ParserError> {
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
                return Ok(());
            } else {
                self.add_text_to_last_inline(newline_token);
                self.dangling_newline = None;
            }
        }
        self.add_text_to_last_inline(token);
        Ok(())
    }

    fn pass_text_through(&mut self, token: Token) -> Result<(), ParserError> {
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
                    InlineLiteral::new_text_from_token_pass(&token),
                ));
                return Ok(());
            } else {
                self.add_text_to_last_inline(newline_token);
                self.dangling_newline = None;
            }
        }
        self.pass_text_to_last_inline(token);
        Ok(())
    }

    fn parse_charref(&mut self, token: Token) -> Result<(), ParserError> {
        let inline_literal = Inline::InlineLiteral(InlineLiteral::new_charref_from_token(&token));
        if let Some(newline_token) = self.dangling_newline.clone() {
            if self.preserve_newline_text {
                // add the newline as such
                self.inline_stack.push_back(Inline::InlineLiteral(
                    InlineLiteral::new_text_from_token(&newline_token),
                ));
                // clear the newline
                self.dangling_newline = None;
                // add the new text separately
                self.inline_stack.push_back(inline_literal);
                return Ok(());
            } else {
                self.add_text_to_last_inline(newline_token);
                self.dangling_newline = None;
            }
        }
        if let Some(last_inline) = self.inline_stack.back_mut() {
            match last_inline {
                Inline::InlineSpan(span) => {
                    if self.in_inline_span {
                        span.add_inline(inline_literal);
                    } else {
                        self.inline_stack.push_back(inline_literal)
                    }
                }
                Inline::InlineRef(inline_ref) => {
                    if !self.close_parent_after_push {
                        inline_ref.inlines.push(inline_literal);
                    } else {
                        self.inline_stack.push_back(inline_literal)
                    }
                }
                _ => {
                    // can't add to the back, so just add the literal
                    self.inline_stack.push_back(inline_literal)
                }
            }
        } else {
            self.inline_stack.push_back(inline_literal)
        }
        Ok(())
    }

    fn parse_delimited_leaf_block(&mut self, token: Token) -> Result<(), ParserError> {
        if self.open_parse_after_as_text_type.is_some() {
            // ensure inlines are added appropriately
            self.add_inlines_to_block_stack()?;
            match self.block_stack.pop() {
                Some(mut open_leaf) => {
                    open_leaf.add_locations(token.locations().clone());
                    self.push_block_to_stack(open_leaf)?;
                    self.open_parse_after_as_text_type = None;
                    Ok(())
                }
                None => Err(ParserError::OpenParse(token.line)),
            }
        } else {
            self.open_parse_after_as_text_type = Some(token.token_type());
            let block = LeafBlock::new_from_token(token)?;
            self.push_block_to_stack(Block::LeafBlock(block))?;
            // note that we're to just add
            self.force_new_block = false;
            Ok(())
        }
    }

    fn parse_delimited_parent_block(&mut self, token: Token) -> Result<(), ParserError> {
        let delimiter_line = token.first_location().line;
        let mut block = ParentBlock::new_from_token(token)?;
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
                return Err(ParserError::ParentBlock(delimiter_line));
            };
            if matched == block {
                // close any dangling inlines BEFORE opening the delimited block lines
                self.add_inlines_to_block_stack()?;
                // remove the open delimiter line from the count and confirm we're nested properly
                let Some(line) = self.open_delimited_block_lines.pop() else {
                    return Err(ParserError::DelimitedBlock);
                };
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
                self.push_block_to_stack(delimited_block)?;
                // close any continuations
                self.in_block_continuation = false;
                return Ok(());
            } else {
                self.block_stack
                    .insert(parent_block_idx, Block::ParentBlock(matched));
            }
        }
        // note the open block line
        self.open_delimited_block_lines.push(delimiter_line);
        self.push_block_to_stack(Block::ParentBlock(block))
    }

    fn parse_table_cell(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        // take the token text, which begins with a `|`, and then use everything after
        let cell_contents = token.text()[1..].to_string();
        let cell_line = token.first_location().line;
        let cell_col = token.first_location().col;

        self.push_block_to_stack(Block::TableCell(TableCell::new_from_token(token)))?;

        // create new inlines from the stack, clearing any dangling newlines
        self.dangling_newline = None;
        for result in Scanner::new(&cell_contents) {
            match result {
                Ok(mut inline_token) => {
                    // update the line numbering for the token to match the table cell mathematically, in
                    // case my vague memory of multi-line cells is correct
                    inline_token.update_token_loc_offsets_by(cell_line, cell_col);
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

    fn push_block_to_stack(&mut self, mut block: Block) -> Result<(), ParserError> {
        // we only want to push on continue if we're not in an open delimited block (which will
        // close itself, emptying the open_delimited_block_lines)
        if self.in_block_continuation && self.open_delimited_block_lines.is_empty() {
            let Some(last_block) = self.block_stack.last_mut() else {
                return Err(ParserError::BlockContinuation);
            };
            last_block.push_block(block)?;
            self.in_block_continuation = false;
        } else {
            if self.metadata.is_some() {
                block.add_metadata(self.metadata.as_ref().unwrap().clone())?;
                self.metadata = None;
            }
            self.block_stack.push(block)
        }
        Ok(())
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
                Inline::InlineLiteral(prior_literal) => {
                    if matches!(prior_literal.name, InlineLiteralName::Charref) {
                        self.inline_stack.push_back(inline_literal)
                    } else {
                        prior_literal.add_text_from_token(&token)
                    }
                }
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

    fn pass_text_to_last_inline(&mut self, token: Token) {
        let inline_literal = Inline::InlineLiteral(InlineLiteral::new_text_from_token_pass(&token));
        if let Some(last_inline) = self.inline_stack.back_mut() {
            match last_inline {
                Inline::InlineSpan(span) => {
                    if self.in_inline_span {
                        span.add_inline(inline_literal);
                    } else {
                        self.inline_stack.push_back(inline_literal)
                    }
                }
                Inline::InlineLiteral(prior_literal) => {
                    if matches!(prior_literal.name, InlineLiteralName::Charref) {
                        self.inline_stack.push_back(inline_literal)
                    } else {
                        prior_literal.pass_text_from_token(&token)
                    }
                }
                Inline::InlineRef(inline_ref) => {
                    if !self.close_parent_after_push {
                        inline_ref.pass_text_from_token(token)
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

    fn handle_dangling_spans(&mut self) {
        // look for the last span in the stack
        if let Some(open_span_idx) = self
            .inline_stack
            .iter()
            .rposition(|inline| inline.is_open())
        {
            let mut open_span = self.inline_stack.remove(open_span_idx).unwrap();
            let open_span_literal = open_span.produce_literal_from_self();
            // put the literal char into the stack as a literal...
            let mut children = open_span.extract_child_inlines();
            // ... by adding it to the next inline
            if let Some(inline) = children.front_mut() {
                match inline {
                    Inline::InlineLiteral(literal) => {
                        literal.prepend_to_value(open_span_literal, open_span.locations());
                    }
                    _ => todo!(),
                }
                // put any appended inlines into the stack at the relevant position
                while children.len() > 0 {
                    if let Some(child) = children.pop_back() {
                        self.inline_stack.insert(open_span_idx, child);
                    }
                }

                // consolidate any resultant or remaining adjacent literals (this should be extracted to a function)
                let mut temp_stack: Vec<Inline> = vec![];
                while let Some(mut inline) = self.inline_stack.pop_front() {
                    if temp_stack.len() == 0 {
                        temp_stack.push(inline);
                    } else if inline.is_literal() {
                        if let Some(Inline::InlineLiteral(last_in_stack)) = temp_stack.last_mut() {
                            last_in_stack.combine_literals(inline.extract_literal());
                        } else {
                            temp_stack.push(inline);
                        }
                    } else {
                        temp_stack.push(inline);
                    }
                }
                // let mut inline_stack_iter = self.inline_stack.iter_mut().peekable();
                // // this doesn't work like it should! it's skipping the middle, because of the
                // // way it's looping
                // while inline_stack_iter.peek().is_some() {
                //     if let Some(current) = inline_stack_iter.next() {
                //         dbg!(&current);
                //         if let Inline::InlineLiteral(current_literal) = current {
                //             if let Some(Inline::InlineLiteral(next_literal)) =
                //                 inline_stack_iter.next()
                //             {
                //                 current_literal.combine_literals(next_literal.clone());
                //             }
                //         }
                //         temp_stack.push_back(current.clone());
                //     }
                // }
                self.inline_stack = temp_stack.into();
            } else {
                // ... or if there are no children, add the token to the back of the last one; this
                // is a little hacky, but it is cleaner compared to the rest of the code just to
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
                    file_stack: self.file_stack.clone(),
                };
                self.add_text_to_last_inline(reconstituted_token)
            }
        }
    }

    fn add_inlines_to_block_stack(&mut self) -> Result<(), ParserError> {
        // guard
        if self.inline_stack.is_empty() {
            return Ok(());
        }

        // dangling inlines
        if self.in_inline_span {
            self.handle_dangling_spans();
        }

        if let Some(header) = &mut self.document_header {
            while !self.inline_stack.is_empty() {
                header.title.push(self.inline_stack.pop_front().unwrap());
            }
            return Ok(());
        }

        if let Some(last_block) = self.block_stack.last_mut() {
            if last_block.takes_inlines() && !self.in_block_line && !self.force_new_block {
                while !self.inline_stack.is_empty() {
                    let inline = self.inline_stack.pop_front().unwrap();
                    last_block.push_inline(inline)?;
                }
                return Ok(());
            }
        }
        // create a new para from the locations of the first span (subsequent locations are
        // consolidated later)
        let mut para_locations: Vec<Location> = Vec::new();
        if let Some(first_inline) = self.inline_stack.front() {
            para_locations = first_inline.locations().clone();
        }
        let mut para_block = Block::LeafBlock(LeafBlock::new(
            crate::graph::blocks::LeafBlockName::Paragraph,
            crate::graph::blocks::LeafBlockForm::Paragraph,
            None,
            para_locations,
            vec![],
        ));
        while !self.inline_stack.is_empty() {
            if let Some(inline) = self.inline_stack.pop_front() {
                para_block.push_inline(inline)?
            }
        }
        if self.in_block_continuation && self.open_delimited_block_lines.is_empty() {
            let Some(last_block) = self.block_stack.last_mut() else {
                return Err(ParserError::BlockContinuation);
            };
            last_block.push_block(para_block)?;
            return Ok(());
        }

        if let Some(ref block_metadata) = self.metadata {
            // check to see if we need to apply metadata to the para block
            let line_above = para_block.locations().first().unwrap().line + 1;
            if self.open_delimited_block_lines.last() == Some(&line_above)
                || self.open_delimited_block_lines.is_empty()
                    && block_metadata.declared_type == Some(AttributeType::Quote)
            {
                let mut quote_block = Block::ParentBlock(ParentBlock::new(
                    crate::graph::blocks::ParentBlockName::Quote,
                    None,
                    "".to_string(),
                    vec![],
                    vec![],
                ));
                quote_block.push_block(para_block)?;
                self.push_block_to_stack(quote_block)?;
                return Ok(());
            }
        }
        self.push_block_to_stack(para_block)
    }

    /// Adds data to an existing ElementMetadata object, or creates one
    fn add_metadata_from_token(&mut self, token: Token) {
        match self.metadata {
            Some(ref mut metadata) => metadata.add_metadata_from_token(token),
            None => {
                self.metadata = Some(ElementMetadata::new_block_meta_from_token(token));
            }
        }
    }

    fn add_last_list_item_to_list(&mut self) -> Result<(), ParserError> {
        // clear out any forced new blocks
        self.force_new_block = false;
        // add the inlines to the list item
        self.add_inlines_to_block_stack()?;
        // then add it to the list
        let last_item = self.block_stack.pop().unwrap();
        // if the last thing is a list item, add it to the list
        if matches!(last_item, Block::ListItem(_) | Block::DListItem(_)) {
            if let Some(list) = self.block_stack.last_mut() {
                list.push_block(last_item)?
            }
        } else {
            // otherwise return the list to the open block stack, and create a new unordered
            // list item
            self.push_block_to_stack(last_item)?;
        }
        Ok(())
    }

    fn add_to_block_stack_or_graph(
        &mut self,
        asg: &mut Asg,
        mut block: Block,
    ) -> Result<(), ParserError> {
        if let Some(last_block) = self.block_stack.last_mut() {
            if last_block.takes_block_of_type(&block) {
                last_block.push_block(block)?;
                return Ok(());
            }
        }
        if self.metadata.is_some() {
            block.add_metadata(self.metadata.as_ref().unwrap().clone())?;
            self.metadata = None;
        }
        asg.push_block(block)?;
        Ok(())
    }

    fn add_last_to_block_stack_or_graph(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
        if let Some(last_block) = self.block_stack.pop() {
            if let Some(prior_block) = self.block_stack.last_mut() {
                if prior_block.takes_block_of_type(&last_block) {
                    prior_block.push_block(last_block)?;
                    return Ok(());
                }
            }
            asg.push_block(last_block)?;
            Ok(())
        } else {
            Err(ParserError::BlockStack)
        }
    }

    fn add_last_block_to_graph(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
        if let Some(mut block) = self.block_stack.pop() {
            if self.metadata.is_some() {
                block.add_metadata(self.metadata.as_ref().unwrap().clone())?;
                self.metadata = None;
            }
            if let Some(next_last_block) = self.block_stack.last_mut() {
                if next_last_block.takes_block_of_type(&block) {
                    next_last_block.push_block(block)?;
                } else {
                    asg.push_block(block)?
                }
            } else {
                asg.push_block(block)?
            }
        }
        Ok(())
    }

    fn check_target(&self, token_line: usize, target: &str) -> Result<(), ParserError> {
        if self.resolve_targets {
            match self.resolve_target(token_line, target) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        } else {
            Ok(())
        }
    }

    fn resolve_target(&self, token_line: usize, target: &str) -> Result<PathBuf, ParserError> {
        if !self.file_stack.is_empty() {
            let mut resolved_target = self.origin_directory.clone();
            // may as well follow the rabbit hole
            for file in self.file_stack.iter() {
                if let Some(parent) = PathBuf::from_str(file).unwrap().parent() {
                    resolved_target.push(parent)
                }
            }
            match resolved_target.join(target).canonicalize() {
                Ok(p) => Ok(p),
                Err(_) => Err(ParserError::TargetResolution(token_line, target.into())),
            }
        } else {
            match self.origin_directory.join(target).canonicalize() {
                Ok(p) => Ok(p),
                Err(_) => Err(ParserError::TargetResolution(token_line, target.into())),
            }
        }
    }
}

fn open_file<P>(filename: P) -> String
where
    P: AsRef<Path> + Into<PathBuf> + Debug,
{
    match fs::read_to_string(&filename) {
        Ok(file_string) => file_string,
        Err(e) => {
            warn!("Unable to read file {:?}: {e}", filename);
            std::process::exit(1)
        }
    }
}
