//! Parses a stream of tokens into an [`Asg`] (Abstract Syntax Graph)
use std::{
    collections::{HashMap, VecDeque},
    env,
    fmt::Debug,
    path::PathBuf,
};

use log::warn;

use crate::errors::{ParserError, ScannerError};
use crate::graph::{
    asg::Asg,
    blocks::{Block, LeafBlock, ParentBlock},
    inlines::{Inline, LineBreak},
    metadata::{AttributeType, ElementMetadata},
    nodes::{Header, Location},
};
use crate::scanner::tokens::{Token, TokenType};

mod blocks;
mod header;
mod includes;
mod inlines;
mod list_context;
mod lists;

use header::HeaderLine;
use list_context::ListContext;

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
    /// which line of the document header is being read
    header_line: HeaderLine,
    /// the text of the author or revision line being read, as written
    header_line_text: String,
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
    /// the files (as file stacks) those delimited blocks opened in: an included file's lines are
    /// counted from 1 too, so a line alone doesn't identify a block
    open_delimited_block_files: Vec<Vec<String>>,
    /// appends text to block or inline regardless of markup, token, etc. (will need to change
    /// if/when we handle code callouts)
    open_parse_after_as_text_type: Option<TokenType>,
    /// tracks the document heading level offset
    level_offset: i32,
    /// designates whether we're to be adding inlines to the previous block until a newline
    in_block_line: bool,
    /// designates whether new literal text should be added to the last span
    in_inline_span: bool,
    /// the line of the last `+` that continued a list item
    continuation_line: Option<usize>,
    /// forces a new block when we add inlines; helps distinguish between adding to section.title
    /// and section.blocks
    force_new_block: bool,
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
        // (or the current directory as `.`, if its path can't be had)
        Self::new(env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
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
            header_line: HeaderLine::Title,
            header_line_text: String::new(),
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
            open_delimited_block_files: vec![],
            level_offset: 0,
            in_block_line: false,
            in_inline_span: false,
            continuation_line: None,
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
                    self.line = token.line;
                    self.token_into(token, &mut asg)?;
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
        // the IDs of sections that weren't given one, before cross references are resolved
        asg.generate_section_ids(&self.document_attributes);
        asg.consolidate();
        Ok(asg)
    }

    /// Parses one token, from the document itself or from an included file, and remembers its
    /// type for the next one
    fn token_into(&mut self, mut token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        if self.document_header.is_some() && self.read_header_line(&token) {
            self.last_token_type = token.token_type();
            return Ok(());
        }
        // if we are not starting with a document-heading acceptable token, get out
        if self.document_header.is_some() && !token.can_be_in_document_header() {
            self.check_and_move_header(asg)?;
        }
        if token.token_type() == TokenType::BlockContinuation {
            self.make_text_if_not_a_continuation(&mut token, asg)?;
        }
        if token.is_text_in_table() && self.in_open_table() {
            token.make_text();
        }
        let token_type = token.token_type();
        self.parse_token(token, asg)?;
        self.last_token_type = token_type;
        Ok(())
    }

    fn parse_token(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
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
        // like Asciidoctor, an attribute or anchor line, or a delimiter, ends a paragraph (and a
        // list item's text, below); the attributes belong to the block after it. (Not inside a
        // verbatim block, whose text is pending until its closing delimiter)
        if matches!(
            token.token_type(),
            TokenType::BlockAnchor
                | TokenType::ElementAttributes
                | TokenType::SidebarBlock
                | TokenType::OpenBlock
                | TokenType::ExampleBlock
                | TokenType::QuoteVerseBlock
                | TokenType::PassthroughBlock
                | TokenType::LiteralBlock
                | TokenType::SourceBlock
                | TokenType::CommentBlock
                | TokenType::Table
        ) && self.open_parse_after_as_text_type.is_none()
            && self.lists.get(self.block_stack.len()).is_none()
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
            | TokenType::UnconstrainedEmphasis => self.parse_inline_span(token),

            // replacements -- done on token.text()
            TokenType::OpenDoubleQuote
            | TokenType::CloseDoubleQuote
            | TokenType::OpenSingleQuote
            | TokenType::CloseSingleQuote => self.parse_text(token),

            // references
            TokenType::AttributeReference => self.parse_attribute_reference(token),
            TokenType::CrossReference => self.parse_cross_reference(token, asg),
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
            | TokenType::Table => self.parse_delimited_parent_block(token, asg),

            // table cells -- note that we just create the cells; up to the backend/template to handle the
            // column-making (for now, so we can just reuse ParentBlock)
            // (the scanner can't tell a `|` in a table from one that isn't)
            TokenType::TableCell if self.in_open_table() => self.parse_table_cell(token, asg),
            TokenType::TableCell => self.parse_text(token),

            TokenType::QuoteVerseBlock => {
                // check if it's verse
                if let Some(metadata) = &self.metadata {
                    if metadata.declared_type == Some(AttributeType::Verse) {
                        self.parse_delimited_leaf_block(token, asg)?;
                        return Ok(());
                    }
                } else if self.open_parse_after_as_text_type.is_some() {
                    self.parse_delimited_leaf_block(token, asg)?;
                    return Ok(());
                }

                self.parse_delimited_parent_block(token, asg)?;
                Ok(())
            }

            // the following should probably be consumed into the above
            TokenType::PassthroughBlock | TokenType::LiteralBlock => {
                self.parse_delimited_leaf_block(token, asg)
            }
            TokenType::SourceBlock => self.parse_delimited_leaf_block(token, asg),
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

            // a `+` that continues a list item (any other has been made text; see
            // `make_text_if_not_a_continuation`)
            TokenType::BlockContinuation => self.parse_block_continuation(token.line),

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
                    self.parse_delimited_leaf_block(token, asg)
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

    /// New line characters are arguably the most significant token "signal" we can get, and as
    /// such the parse function is a little complicated.
    fn parse_new_line_char(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        // a line of the document header ends (a blank line ends the header, below)
        if self.document_header.is_some()
            && !matches!(
                self.last_token_type,
                TokenType::NewLineChar | TokenType::Eof
            )
        {
            self.in_block_line = false;
            self.dangling_newline = None;
            return self.end_header_line();
        }
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
        } else if self.last_token_type == TokenType::NewLineChar && self.in_open_table() {
            // a cell's text goes on past a blank line (as one paragraph, since cells can't
            // hold several)
        } else if [TokenType::NewLineChar, TokenType::Eof].contains(&self.last_token_type) {
            // clear any dangling newline
            self.dangling_newline = None;
            if self.document_header.is_some() {
                self.check_and_move_header(asg)?;
            } else if self.lists.active(self.block_stack.len()).is_some() {
                // a blank line ends the current list item's text, but not the list: the next
                // line that isn't blank decides that (see `end_list_after_blank_line`)
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
        self.add_inlines_to_block_stack()?;
        self.in_inline_span = false;
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
            token.make_text();
        }
        token
    }

    /// A `+` line only continues a list item. Elsewhere (directly after another `+` included),
    /// like Asciidoctor, it is text: it ends the paragraph before it, and starts one that takes
    /// the lines after it. (In a table or a verse it is more of the text.)
    fn make_text_if_not_a_continuation(
        &mut self,
        token: &mut Token,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        match self.open_parse_after_as_text_type {
            None => {
                let waiting_for_block = self.inline_stack.is_empty();
                let follows_continuation =
                    self.continuation_line.map(|line| line + 1) == Some(token.line);
                let in_list = match self.lists.get(self.block_stack.len()) {
                    // a `+` directly after one that is still waiting for its block is that
                    // block's text
                    Some(scope)
                        if !(scope.attach_next && waiting_for_block && follows_continuation) =>
                    {
                        return Ok(());
                    }
                    scope => scope.is_some(),
                };
                if !in_list && !self.in_open_table() {
                    self.add_inlines_to_block_stack()?;
                    self.emit_completed_block(asg)?;
                    self.dangling_newline = None;
                    self.force_new_block = true;
                }
            }
            Some(TokenType::QuoteVerseBlock) => {}
            // inside a verbatim block, `parse_token` passes it through
            Some(_) => return Ok(()),
        }
        token.make_text();
        Ok(())
    }

    /// Whether a paragraph's text is waiting on the inline stack (section and block titles are
    /// complete at the end of their lines, so pending text at the start of a line is a paragraph's)
    fn in_paragraph(&self) -> bool {
        !self.inline_stack.is_empty() && self.document_header.is_none() && !self.section_title_open
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

    /// Pushes a block onto the stack, with any pending metadata
    fn push_block_to_stack(&mut self, mut block: Block) -> Result<(), ParserError> {
        if let Some(metadata) = self.metadata.take() {
            block.add_metadata(metadata)?;
        }
        self.block_stack.push(block);
        Ok(())
    }

    /// Puts a block that has just closed (a delimited block) where it belongs: in the current
    /// list item if a `+` attached it; inside a delimited block, on the stack, where the closing
    /// delimiter collects it; and otherwise in its parent (or the graph) right away, so that the
    /// text after it starts a new paragraph rather than joining the block
    fn complete_block(&mut self, block: Block, asg: &mut Asg) -> Result<(), ParserError> {
        let block = self.wrap_in_admonition(block)?;
        let Some(block) = self.attach_to_list_item(block)? else {
            return Ok(());
        };
        if self.open_delimited_block_lines.is_empty() {
            self.force_new_block = true;
            self.add_to_block_stack_or_graph(asg, block)
        } else {
            self.push_block_to_stack(block)
        }
    }

    fn add_inlines_to_block_stack(&mut self) -> Result<(), ParserError> {
        // guard
        if self.inline_stack.is_empty() {
            return Ok(());
        }

        // spans (and link macros) that were never closed are text, their markup included
        while self.inline_stack.iter().any(|inline| inline.is_unclosed()) {
            self.handle_dangling_spans(true);
        }
        self.in_inline_span = false;

        if let Some(header) = &mut self.document_header {
            while !self.inline_stack.is_empty() {
                header.title.push(self.inline_stack.pop_front().unwrap());
            }
            return Ok(());
        }

        // the current list item's principal text
        let depth = self.block_stack.len();
        // a table cell takes all the text up to the next cell
        if let Some(cell @ Block::TableCell(_)) = self.block_stack.last_mut() {
            while let Some(inline) = self.inline_stack.pop_front() {
                cell.push_inline(inline)?;
            }
            return Ok(());
        }
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
        self.warn_unsupported_block_macro(&para_block);
        // a styled paragraph: an admonition (`NOTE: text`), or a quote
        let mut block = para_block;
        let mut title_dropped = pending_title.clone();
        if let Some(admonition) = self.pending_admonition.take() {
            let mut admonition = Block::ParentBlock(admonition);
            // the attributes above are the admonition's
            if let Some(metadata) = self.metadata.take() {
                admonition.add_metadata(metadata)?;
            }
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
        self.push_block_to_stack(block)
    }

    /// A block macro we don't support is a paragraph's text, so let the user know
    fn warn_unsupported_block_macro(&self, para_block: &Block) {
        let Block::LeafBlock(paragraph) = para_block else {
            return;
        };
        let inlines = paragraph.inlines();
        let Some(first) = inlines.first() else { return };
        let text = para_block.block_text();
        let line = text.lines().next().unwrap_or_default().trim_end();
        let first_text = first.extract_values_to_string();
        for name in ["toc", "audio", "video"] {
            let prefix = format!("{name}::");
            // a URL target is a link of its own, which takes the brackets
            let url_target =
                first_text == prefix && matches!(inlines.get(1), Some(Inline::InlineRef(_)));
            let local_target = line
                .strip_prefix(&prefix)
                .is_some_and(|rest| !rest.starts_with(' ') && rest.ends_with(']'));
            if url_target || local_target {
                warn!(
                    "Line {}: The {name}:: block macro is not supported; it is output as text",
                    para_block.locations().first().map_or(0, |start| start.line)
                );
            }
        }
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
}
