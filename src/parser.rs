//! Parses a stream of tokens into an [`Asg`] (Abstract Syntax Graph)
use std::{
    collections::{HashMap, VecDeque},
    env,
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use log::{error, info, warn};

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

mod list_context;
use list_context::{ListContext, marker_key};

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
    /// the open lists, kept off the block stack until they are complete
    lists: ListContext,
    /// holding ground for inline elements until it's time to push to the relevant block
    inline_stack: VecDeque<Inline>,
    /// holding ground for includes file names; if inside an include push to stack, popping off
    /// once the file's tokens have been accommodated (this allows for simpler nesting)
    file_stack: Vec<String>,
    /// holding ground for a block title, to be applied to the subsequent block
    block_title: Option<Vec<Inline>>,
    /// true only while we are still reading the title line itself; once its newline arrives, the
    /// title is complete and later inlines belong to the block it precedes
    block_title_open: bool,
    /// true while we are reading a section heading's line; its newline completes the title, so
    /// that the next line starts the section's first block
    section_title_open: bool,
    /// an admonition (`NOTE: text`, or `[NOTE]`) waiting for its paragraph or block
    pending_admonition: Option<ParentBlock>,
    /// admonitions (`[NOTE]`) around delimited blocks that are still open, with the depth of the
    /// block stack where the block opened
    admonitions_around_blocks: Vec<(usize, ParentBlock)>,
    /// the line the pending block title was declared on
    block_title_line: usize,
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
    /// Preserves newline characters as separate inline literal tokens. Set by the first
    /// description list item, whose fixtures expect it, and never unset
    preserve_newline_text: bool,
    /// Some parent elements have non-obvious closing conditions, so we want an easy way to close these
    close_parent_after_push: bool,
    /// Used to see if we need to add a newline before new text; we don't add newlines to the text
    /// literals unless they're continuous (i.e., we never count newline paras as paras)
    dangling_newline: Option<Token>,
    /// Track lines for better error handling
    line: usize,
    /// Flag for whether or not the parser will require any targets (images, included files, etc.)
    /// to exist to successfully parse the document
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
            lists: ListContext::default(),
            inline_stack: VecDeque::new(),
            file_stack: vec![],
            block_title: None,
            block_title_open: false,
            section_title_open: false,
            pending_admonition: None,
            admonitions_around_blocks: vec![],
            block_title_line: 0,
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
            line: 1,
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
                    self.line = token.line;
                    self.token_into(token, &mut asg)?;

                    self.last_token_type = token_type;
                }
                Err(e) => return Err(ParserError::Scanner(e)),
            }
        }

        // add any dangling inlines
        self.add_inlines_to_block_stack()?;
        self.add_empty_admonition(&mut asg)?;
        // and any open lists
        self.close_lists(&mut asg)?;
        // add any dangling blocks (most often sections); a block left open in a list item (an
        // unclosed delimited block, say) goes to its item
        while !self.block_stack.is_empty() {
            self.add_last_block_to_graph(&mut asg)?;
        }
        // which leaves the lists at the top level open (inside an unclosed delimited block, a
        // closed list goes onto the stack, so it is drained again)
        self.close_lists(&mut asg)?;
        while !self.block_stack.is_empty() {
            self.add_last_block_to_graph(&mut asg)?;
        }
        debug_assert!(
            self.lists.is_empty(),
            "Lists left open inside closed blocks"
        );
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

        let token = self.as_text_if_it_cannot_interrupt(token);
        // like Asciidoctor, an attribute or anchor line ends a paragraph (and a list item's text,
        // below); the attributes belong to the block after it
        if matches!(
            token.token_type(),
            TokenType::BlockAnchor | TokenType::ElementAttributes
        ) && self.lists.get(self.block_stack.len()).is_none()
            && self.in_paragraph()
        {
            self.add_inlines_to_block_stack()?;
            self.emit_completed_block(asg)?;
            self.dangling_newline = None;
        }
        self.end_list_after_blank_line(token.token_type(), asg)?;
        // the start of a block ends the current list item's text, and the list too, unless a `+`
        // attached the block to the item
        if token.token_type().ends_list_item() {
            self.end_list_item_at_block(asg)?;
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
                self.block_title_open = true;
                self.block_title_line = token.line;
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
            TokenType::StartTag | TokenType::EndTag => Ok(()),

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
            TokenType::LineContinuation => {
                self.inline_stack
                    .push_back(Inline::InlineBreak(LineBreak::new_from_token(token)));
                Ok(())
            }
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
            TokenType::UnorderedListItem
            | TokenType::OrderedListItem
            | TokenType::CodeCalloutListItem => self.parse_list_item(token, asg),
            TokenType::DescriptionListMarker => self.parse_description_list_term(token, asg),

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
            // the scanner does not produce these (yet), so report them rather than guess
            TokenType::Email => Err(ParserError::UnexpectedToken(
                token.line,
                "email address".to_string(),
            )),
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
                    self.line,
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
        if let Some(header) = &mut self.document_header
            && !header.is_empty()
        {
            header.consolidate();
            asg.add_header(header.clone(), self.document_attributes.clone())
        }

        self.document_header = None;
        Ok(())
    }

    /// New line characters are arguably the most significant token "signal" we can get, and as
    /// such the parse function is a little complicated.
    fn parse_new_line_char(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        // newline exits a title, TK line continuation
        self.in_block_line = false;

        // if we are on a block title line, add the inline stack to the title; the newline closes
        // the title, so that subsequent lines are not swallowed by it
        let closes_title = self.block_title_open;
        if closes_title {
            if let Some(ref mut title_stack) = self.block_title {
                while !self.inline_stack.is_empty() {
                    let inline = self.inline_stack.pop_front().unwrap();
                    title_stack.push(inline);
                }
            }
            self.block_title_open = false;
        }
        let closes_section_title = self.section_title_open;
        if closes_section_title {
            self.add_inlines_to_block_stack()?;
            self.section_title_open = false;
            self.force_new_block = true;
        }

        if self.last_token_type == TokenType::NewLineChar
            && self.open_parse_after_as_text_type == Some(TokenType::QuoteVerseBlock)
        {
            // a blank line inside a verse block separates stanzas; it is part of the verse's text,
            // not the end of a paragraph
            if let Some(newline_token) = self.dangling_newline.take() {
                self.add_text_to_last_inline(newline_token);
            }
            self.add_text_to_last_inline(token);
        } else if [TokenType::NewLineChar, TokenType::Eof].contains(&self.last_token_type) {
            // clear any dangling newline
            self.dangling_newline = None;
            if self.document_header.is_some() {
                self.check_and_move_header(asg)?;
            } else if self.lists.active(self.block_stack.len()).is_some() {
                // a blank line ends the current list item's text, but not the list: the next
                // line that isn't blank decides that (see `end_list_after_blank_line`)
                self.in_inline_span = false;
                self.add_inlines_to_block_stack()?;
                if let Some(scope) = self.lists.active(self.block_stack.len()) {
                    scope.blank_line();
                }
                self.force_new_block = true;
            } else {
                self.end_blocks_at_blank_line(asg)?;
            }
        } else if closes_title
            || closes_section_title
            || self.in_block_continuation
            || self.last_token_type.clears_newline_after()
        {
            // don't add a newline ahead of text in these cases (including the newline that ends
            // a block title line, which would otherwise be glued onto the following block's text)
            self.dangling_newline = None;
        } else {
            self.dangling_newline = Some(token);
        }
        Ok(())
    }

    /// A blank line completes the open paragraph, and the text after it starts a new block
    fn end_blocks_at_blank_line(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
        // clear out any inlines
        self.in_inline_span = false;
        self.add_inlines_to_block_stack()?;
        self.add_empty_admonition(asg)?;
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
        }
        Ok(())
    }

    /// Like Asciidoctor, a list item, a section heading or a block title can't interrupt a line of
    /// text, so it is text if it directly follows a paragraph line. Nor can a heading interrupt a list, unless a
    /// blank line came before it: it is text in the item (a list item can't contain a section).
    fn as_text_if_it_cannot_interrupt(&self, mut token: Token) -> Token {
        let is_heading = matches!(
            token.token_type(),
            TokenType::Heading1
                | TokenType::Heading2
                | TokenType::Heading3
                | TokenType::Heading4
                | TokenType::Heading5
        );
        let is_list_item = matches!(
            token.token_type(),
            TokenType::UnorderedListItem | TokenType::OrderedListItem
        );
        let is_block_title = token.token_type() == TokenType::BlockLabel;
        if !is_heading && !is_list_item && !is_block_title {
            return token;
        }
        let as_text = match self.lists.get(self.block_stack.len()) {
            Some(scope) => is_heading && !scope.after_blank,
            None => self.in_paragraph(),
        };
        if as_text {
            token.token_type = TokenType::Text;
            token.literal = Some(token.lexeme.clone());
        }
        token
    }

    /// Whether a paragraph's text is waiting on the inline stack (section and block titles are
    /// complete at the end of their lines, so pending text at the start of a line is a paragraph's)
    fn in_paragraph(&self) -> bool {
        !self.inline_stack.is_empty() && self.document_header.is_none() && !self.section_title_open
    }

    /// After a blank line in a list, the next line that isn't blank decides whether the list
    /// goes on: an item or a `+` continues it, and anything else ends it. Ending it then does
    /// what the blank line would have done without a list.
    ///
    /// A line of text is only known not to be a description list item (whose terms come before
    /// the `::`) once it ends without a `::`, so text waits on the inline stack until then.
    fn end_list_after_blank_line(
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
        info!("Parsing included file: {}", &target);
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
                if let Some(line) = included_lines.last()
                    && *line == -1
                {
                    include_to_end = true;
                    included_lines.pop();
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
                        if token.token_type() == TokenType::StartTag
                            && let Some(tag) = token.tag()
                            && included_tags.contains(&tag)
                        {
                            current_tag = Some(tag);
                            continue;
                        }
                        if token.token_type() == TokenType::EndTag
                            && let Some(tag) = token.tag()
                            && current_tag == Some(tag)
                        {
                            current_tag = None;
                            continue;
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

    /// Starts a description list item, which works like any other list item (see
    /// [`parse_list_item`](Self::parse_list_item)). Its terms are the text on the marker's line;
    /// the text on the lines before it belongs to what came before (the previous item, say).
    fn parse_description_list_term(
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
    fn parse_list_item(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
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

    /// Outside a delimited block, a block left complete on top of the stack (a paragraph, say,
    /// or a closed listing) goes to its parent before the next block (a list, say) starts, so that
    /// the next block comes after it rather than inside it. Sections stay, since they hold the list. (Inside a delimited
    /// block, completed blocks wait on the stack for the closing delimiter, which collects them
    /// in order.)
    fn emit_completed_block(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
        if self.open_delimited_block_lines.is_empty()
            && self
                .block_stack
                .last()
                .is_some_and(|block| !block.is_section())
        {
            self.add_last_to_block_stack_or_graph(asg)?;
        }
        Ok(())
    }

    /// Called at a token that starts a block. If a `+` came just before, the block belongs to the
    /// current list item, which takes no more text. Otherwise the item's text (or the paragraph
    /// attached to it) is complete, and so is the list.
    fn end_list_item_at_block(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
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
    fn close_lists(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
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
        self.section_title_open = true;
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
                    self.line,
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
    fn drop_admonition_above_list(&mut self) {
        self.pending_admonition = None;
    }

    /// An admonition with nothing after it (`NOTE:` then a blank line) is empty
    fn add_empty_admonition(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
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
    fn wrap_in_admonition(&mut self, block: Block) -> Result<Block, ParserError> {
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

    fn parse_block_continuation(&mut self, _token: Token) -> Result<(), ParserError> {
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

    /// Generic parser for inline spans that close themselves
    fn parse_inline_span(&mut self, mut inline: Inline) -> Result<(), ParserError> {
        // Only a span that is still open can be closed by, or contain, the incoming span. A newline
        // is held back as "dangling" rather than pushed as text, so after "*a*\n" the closed
        // strong span is still the last inline on the stack; treating it as open would swallow a
        // span that begins on the next line.
        if let Some(last_inline) = self.inline_stack.back_mut()
            && last_inline.is_open()
        {
            if inline == *last_inline {
                last_inline.reconcile_locations(inline.locations());
                last_inline.close();
                self.in_inline_span = false;
                return Ok(());
            } else if let Inline::InlineSpan(last_span) = last_inline {
                if let Some(last_internal_inline) = last_span.inlines.last_mut()
                    && last_internal_inline.is_open()
                    && inline == *last_internal_inline
                {
                    last_internal_inline.reconcile_locations(inline.locations());
                    last_internal_inline.close();
                } else {
                    // Like Asciidoctor, a newline ahead of a nested span is text in the
                    // enclosing span, so flush it before the nested span starts
                    if let Some(newline_token) = self.dangling_newline.take() {
                        last_span.add_inline(Inline::InlineLiteral(
                            InlineLiteral::new_text_from_token(&newline_token),
                        ));
                    }
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
        let link = Inline::InlineRef(InlineRef::new_link_from_token(token));
        // a bare URL inside an open span (`*see http://x.com*`) belongs to the span
        if let Some(Inline::InlineSpan(span)) = self.inline_stack.back_mut()
            && span.open
        {
            span.add_inline(link);
        } else {
            self.inline_stack.push_back(link);
        }
        Ok(())
    }

    fn parse_block_image(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        let (target, metadata) = target_and_attrs_from_token(&token);
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

    fn parse_inline_image_macro(&mut self, token: Token) -> Result<(), ParserError> {
        let (target, metadata) = target_and_attrs_from_token(&token);
        // as with block images: report the missing file, but still render the image
        if let Err(e) = self.check_target(token.line, &target) {
            warn!("{e}");
        }
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
            .rposition(|inline| inline.is_open_macro())
        {
            // consolidate into the inline macro, which is then complete
            let mut inline_macro = self.inline_stack.remove(inline_macro_idx).unwrap();
            // the macro is complete, so a later `]` (or span) is not its
            match &mut inline_macro {
                Inline::InlineRef(iref) => iref.open = false,
                Inline::InlineSpan(span) => span.open = false,
                _ => {}
            }
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
        if let Some(value) = self.document_attributes.get("icons")
            && value == "true"
            && let Some(Inline::InlineLiteral(last_literal)) = self.inline_stack.back_mut()
        {
            // when we're using icons, the comment markup ahead of the callout goes away
            last_literal.strip_trailing_comment_markup();
        }
        self.inline_stack
            .push_back(Inline::InlineSpan(InlineSpan::inline_span_from_token(
                token,
            )));
        Ok(())
    }

    fn parse_text(&mut self, mut token: Token) -> Result<(), ParserError> {
        // like Asciidoctor, the indentation of a list item's lines is not part of its text (a
        // paragraph's, or an attached paragraph's, is): its lazy lines, or the line after a term
        // that ends its line
        if (self.dangling_newline.is_some() || self.inline_stack.is_empty())
            && self
                .lists
                .get(self.block_stack.len())
                .is_some_and(|scope| scope.principal_open)
        {
            // nor does the text start with the newline before it
            if self.inline_stack.is_empty() {
                self.dangling_newline = None;
            }
            let text = token.text();
            let indent = text.len() - text.trim_start().len();
            if indent == text.len() {
                // nothing but indentation; the line's text is in the tokens that follow
                return Ok(());
            }
            if indent > 0 {
                token.literal = Some(text[indent..].to_string());
                token.startcol += indent;
            }
        }
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
        // text inside an inline passthrough macro is kept apart from regular text, as it must not
        // be escaped when output
        let raw = matches!(
            self.open_parse_after_as_text_type,
            Some(TokenType::PassthroughInlineMacro)
        );
        self.pass_text_to_last_inline(token, raw);
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
                    if inline_ref.open {
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
                    self.complete_block(open_leaf)?;
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
            self.open_block(Block::LeafBlock(block))?;
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
                self.complete_block(delimited_block)?;
                // close any continuations
                self.in_block_continuation = false;
                return Ok(());
            }
        }
        // note the open block line
        self.open_delimited_block_lines.push(delimiter_line);
        self.open_pending_admonition();
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
                    // the cell is scanned as if it started a line, but list markup is only
                    // parsed in AsciiDoc ("a|") cells, which we don't support, so `| 1. b` is text,
                    // and so are description list markers and admonition labels
                    if matches!(
                        inline_token.token_type(),
                        TokenType::UnorderedListItem
                            | TokenType::OrderedListItem
                            | TokenType::DescriptionListMarker
                            | TokenType::NotePara
                            | TokenType::TipPara
                            | TokenType::ImportantPara
                            | TokenType::CautionPara
                            | TokenType::WarningPara
                    ) {
                        inline_token.token_type = TokenType::Text;
                        inline_token.literal = Some(inline_token.lexeme.clone());
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

    fn push_block_to_stack(&mut self, block: Block) -> Result<(), ParserError> {
        // we only want to push on continue if we're not in an open delimited block (which will
        // close itself, emptying the open_delimited_block_lines)
        if self.in_block_continuation && self.open_delimited_block_lines.is_empty() {
            let Some(last_block) = self.block_stack.last_mut() else {
                return Err(ParserError::BlockContinuation(self.line));
            };
            last_block.push_block(block)?;
            self.in_block_continuation = false;
            Ok(())
        } else {
            self.open_block(block)
        }
    }

    /// Pushes a block onto the stack, with any pending metadata
    fn open_block(&mut self, mut block: Block) -> Result<(), ParserError> {
        if let Some(metadata) = self.metadata.take() {
            block.add_metadata(metadata)?;
        }
        self.block_stack.push(block);
        Ok(())
    }

    /// Puts a block that has just closed (a delimited block) where it belongs: in the current
    /// list item if a `+` attached it, otherwise where `push_block_to_stack` puts it
    fn complete_block(&mut self, block: Block) -> Result<(), ParserError> {
        let block = self.wrap_in_admonition(block)?;
        match self.attach_to_list_item(block)? {
            Some(block) => self.push_block_to_stack(block),
            None => Ok(()),
        }
    }

    /// A block that is complete while a `+` waits for one belongs to the current list item.
    /// Returns the block if no item is waiting for it.
    fn attach_to_list_item(&mut self, block: Block) -> Result<Option<Block>, ParserError> {
        match self.lists.active(self.block_stack.len()) {
            Some(scope) if scope.attach_next => {
                scope.attach_block(block)?;
                Ok(None)
            }
            _ => Ok(Some(block)),
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
                Inline::InlineLiteral(prior_literal) => {
                    if matches!(
                        prior_literal.name,
                        InlineLiteralName::Charref | InlineLiteralName::Raw
                    ) {
                        // character references and passthroughs stay on their own
                        self.inline_stack.push_back(inline_literal)
                    } else {
                        prior_literal.add_text_from_token(&token)
                    }
                }
                Inline::InlineRef(inline_ref) => {
                    if inline_ref.open {
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

    fn pass_text_to_last_inline(&mut self, token: Token, raw: bool) {
        let inline_literal = Inline::InlineLiteral(if raw {
            InlineLiteral::new_raw_from_token(&token)
        } else {
            InlineLiteral::new_text_from_token_pass(&token)
        });
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
                    let same_kind = match prior_literal.name {
                        InlineLiteralName::Raw => raw,
                        InlineLiteralName::Text => !raw,
                        InlineLiteralName::Charref => false,
                    };
                    if same_kind {
                        prior_literal.pass_text_from_token(&token)
                    } else {
                        // passthrough text must not be mixed into regular text, or vice versa
                        self.inline_stack.push_back(inline_literal)
                    }
                }
                Inline::InlineRef(inline_ref) => {
                    if inline_ref.open {
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
            // without a closing mark, the span's markup is just text, which goes in front of
            // whatever the span held
            let mut children = open_span.children_with_markup_as_text();
            // put the inlines into the stack at the relevant position
            while let Some(child) = children.pop_back() {
                self.inline_stack.insert(open_span_idx, child);
            }

            // consolidate any resultant or remaining adjacent literals, but never merge
            // passthrough (raw) content with text, since text is escaped and passthroughs aren't
            let mut temp_stack: Vec<Inline> = vec![];
            while let Some(inline) = self.inline_stack.pop_front() {
                if let (Some(Inline::InlineLiteral(last_in_stack)), Inline::InlineLiteral(literal)) =
                    (temp_stack.last_mut(), &inline)
                    && last_in_stack.can_merge_with(literal)
                {
                    last_in_stack.combine_literals(literal.clone());
                } else {
                    temp_stack.push(inline);
                }
            }
            self.inline_stack = temp_stack.into();
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

        // the current list item's principal text
        let depth = self.block_stack.len();
        if let Some(scope) = self.lists.active(depth)
            && scope.principal_open
        {
            while let Some(inline) = self.inline_stack.pop_front() {
                scope.push_principal(inline)?;
            }
            return Ok(());
        }

        if let Some(last_block) = self.block_stack.last_mut()
            && last_block.takes_inlines()
            && !self.in_block_line
            && !self.force_new_block
        {
            while !self.inline_stack.is_empty() {
                let inline = self.inline_stack.pop_front().unwrap();
                last_block.push_inline(inline)?;
            }
            return Ok(());
        }
        // create a new para from the locations of the first span (subsequent locations are
        // consolidated later)
        let mut para_locations: Vec<Location> = Vec::new();
        if let Some(first_inline) = self.inline_stack.front() {
            para_locations = first_inline.locations().clone();
        }
        // a pending title is dealt with below, depending on what the paragraph turns into
        let pending_title = self.block_title.take();
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
        // a styled paragraph: an admonition (`NOTE: text`), or a quote
        let mut block = para_block;
        let mut title_dropped = pending_title.clone();
        if let Some(admonition) = self.pending_admonition.take() {
            // the attributes above stay with the paragraph, as they always have (see the roadmap)
            if let Some(metadata) = self.metadata.take() {
                block.add_metadata(metadata)?;
            }
            let mut admonition = Block::ParentBlock(admonition);
            admonition.push_block(block)?;
            block = admonition;
        } else if let Some(ref block_metadata) = self.metadata {
            // check to see if we need to apply metadata to the para block
            let line_above = block.locations().first().map_or(0, |loc| loc.line) + 1;
            if self.open_delimited_block_lines.last() == Some(&line_above)
                || self.open_delimited_block_lines.is_empty()
                    && block_metadata.declared_type == Some(AttributeType::Quote)
            {
                let mut quote = ParentBlock::new(
                    crate::graph::blocks::ParentBlockName::Quote,
                    None,
                    "".to_string(),
                    vec![],
                    vec![],
                );
                // the title belongs to the quote, not the paragraph inside it
                if let Some(title) = title_dropped.take() {
                    quote.title = title;
                }
                let mut quote = Block::ParentBlock(quote);
                quote.push_block(block)?;
                block = quote;
            }
        }
        if let Block::LeafBlock(_) = block {
            self.warn_paragraph_title_dropped(title_dropped.as_deref(), &block);
        }

        // past its principal text, a paragraph in a list belongs to the current item: it follows
        // a `+`, or a block that a `+` attached
        if self.lists.active(depth).is_some() {
            if let Some(metadata) = self.metadata.take() {
                block.add_metadata(metadata)?;
            }
            if let Some(scope) = self.lists.active(depth) {
                scope.attach_block(block)?;
            }
            return Ok(());
        }
        if self.in_block_continuation && self.open_delimited_block_lines.is_empty() {
            let Some(last_block) = self.block_stack.last_mut() else {
                return Err(ParserError::BlockContinuation(self.line));
            };
            last_block.push_block(block)?;
            return Ok(());
        }
        self.push_block_to_stack(block)
    }

    /// Titles on regular paragraphs are not supported, so we drop them (keeping the paragraph),
    /// and let the user know. We only warn when the title sits directly above the paragraph; a
    /// title left over from some other kind of block (e.g., a list) is dropped quietly.
    fn warn_paragraph_title_dropped(&self, title: Option<&[Inline]>, para_block: &Block) {
        let Some(title) = title else { return };
        let adjacent = para_block
            .locations()
            .first()
            .is_some_and(|loc| loc.line == self.block_title_line + 1);
        if adjacent {
            let title_text: String = title.iter().map(|i| i.extract_values_to_string()).collect();
            warn!(
                "Line {}: Titles on paragraphs are not supported; dropping title \"{}\"",
                self.block_title_line, title_text
            );
        }
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

    fn add_to_block_stack_or_graph(
        &mut self,
        asg: &mut Asg,
        mut block: Block,
    ) -> Result<(), ParserError> {
        if self
            .lists
            .get(self.block_stack.len())
            .is_some_and(|scope| scope.attach_next)
        {
            if let Some(metadata) = self.metadata.take() {
                block.add_metadata(metadata)?;
            }
            return self.attach_to_list_item(block).map(|_| ());
        }
        if let Some(last_block) = self.block_stack.last_mut()
            && last_block.takes_block_of_type(&block)
        {
            last_block.push_block(block)?;
            return Ok(());
        }
        if self.metadata.is_some() {
            block.add_metadata(self.metadata.as_ref().unwrap().clone())?;
            self.metadata = None;
        }
        asg.push_block(block)?;
        Ok(())
    }

    fn add_last_to_block_stack_or_graph(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
        // lists inside the block are complete before the block is
        self.close_lists(asg)?;
        if let Some(last_block) = self.block_stack.pop() {
            let last_block = self.wrap_in_admonition(last_block)?;
            let Some(last_block) = self.attach_to_list_item(last_block)? else {
                return Ok(());
            };
            if let Some(prior_block) = self.block_stack.last_mut()
                && prior_block.takes_block_of_type(&last_block)
            {
                prior_block.push_block(last_block)?;
                return Ok(());
            }
            asg.push_block(last_block)?;
            Ok(())
        } else {
            Err(ParserError::BlockStack(self.line))
        }
    }

    fn add_last_block_to_graph(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
        // lists inside the block are complete before the block is
        self.close_lists(asg)?;
        if let Some(mut block) = self.block_stack.pop() {
            if self.metadata.is_some() {
                block.add_metadata(self.metadata.as_ref().unwrap().clone())?;
                self.metadata = None;
            }
            let block = self.wrap_in_admonition(block)?;
            let Some(block) = self.attach_to_list_item(block)? else {
                return Ok(());
            };
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
