//! Inlines: text, spans, links, cross references, footnotes, images and other inline macros

use log::warn;

use crate::scanner::{Token, TokenType};
use crate::{errors::ParserError, scanner::Scanner};
use crate::{
    graph::metadata::target_and_attrs_from_token,
    graph::{
        asg::Asg,
        inlines::{Inline, InlineLiteral, InlineLiteralName, InlineRef, InlineSpan},
        metadata::ElementMetadata,
    },
};

use super::Parser;

impl Parser {
    pub(super) fn parse_inline_element_attributes(
        &mut self,
        token: Token,
    ) -> Result<(), ParserError> {
        self.metadata = Some(ElementMetadata::new_inline_meta_from_token(token));
        self.force_new_block = true;
        Ok(())
    }

    /// Generic parser for inline spans that close themselves. A mark closes the open span of
    /// its kind, however deeply the spans inside that one are nested. A mark that can't close
    /// is text if a span of its kind is already open (so there is only ever one of each, and
    /// they can't nest without limit)
    pub(super) fn parse_inline_span(&mut self, token: Token) -> Result<(), ParserError> {
        let can_close = token.can_close;
        let mut inline = Inline::InlineSpan(InlineSpan::inline_span_from_token(token.clone()));
        // Only a span that is still open can be closed by, or contain, the incoming span. A newline
        // is held back as "dangling" rather than pushed as text, so after "*a*\n" the closed
        // strong span is still the last inline on the stack; treating it as open would swallow a
        // span that begins on the next line.
        if let Some(last_inline) = self.inline_stack.back_mut()
            && last_inline.is_open()
        {
            if can_close && inline == *last_inline {
                last_inline.reconcile_locations(inline.locations());
                last_inline.close();
                self.in_inline_span = false;
                return Ok(());
            }
            if let (Inline::InlineSpan(last_span), Inline::InlineSpan(mark)) =
                (&mut *last_inline, &inline)
            {
                if let Some(depth) = last_span.open_depth_of(mark) {
                    if !can_close {
                        return self.parse_text(token);
                    }
                    last_span.close_at_depth(depth, inline.locations());
                    return Ok(());
                }
                // Like Asciidoctor, a newline ahead of a nested span is text in the
                // enclosing span, so flush it before the nested span starts
                if let Some(newline_token) = self.dangling_newline.take() {
                    last_span.add_inline(Inline::InlineLiteral(
                        InlineLiteral::new_text_from_token(&newline_token),
                    ));
                }
                last_span.add_inline(inline);
                return Ok(());
            }
        }
        // handle newline tokens prior to constrained spans
        if let Some(newline_token) = self.dangling_newline.clone() {
            self.add_text_to_last_inline(newline_token);
            self.dangling_newline = None;
        }
        // check for any other dangling spans (otherwise they disappear!)
        self.handle_dangling_spans(false);
        // add any metadata to new inline
        if self.metadata.is_some() {
            inline.add_metadata(self.metadata.as_ref().unwrap().clone());
            self.metadata = None;
        }
        self.inline_stack.push_back(inline);

        self.in_inline_span = true;
        Ok(())
    }

    pub(super) fn parse_link_macro(&mut self, token: Token) -> Result<(), ParserError> {
        let link = Inline::InlineRef(InlineRef::new_link_from_macro_token(token));
        self.push_link(link);
        Ok(())
    }

    pub(super) fn parse_hyperlink(&mut self, token: Token) -> Result<(), ParserError> {
        let link = Inline::InlineRef(InlineRef::new_link_from_token(token));
        self.push_link(link);
        Ok(())
    }

    /// A link goes after the text before it (including the newline that ended the line before,
    /// which is text, not the link's), inside the span it's in (`` `link:x[y]` ``,
    /// `*see http://x.com*`), if any
    fn push_link(&mut self, link: Inline) {
        if let Some(newline_token) = self.dangling_newline.take() {
            self.add_text_to_last_inline(newline_token);
        }
        if let Some(Inline::InlineSpan(span)) = self.inline_stack.back_mut()
            && span.open
        {
            span.add_inline(link);
        } else {
            self.inline_stack.push_back(link);
        }
    }

    pub(super) fn parse_inline_image_macro(&mut self, token: Token) -> Result<(), ParserError> {
        let (target, metadata) = target_and_attrs_from_token(&token)?;
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

    pub(super) fn parse_footnote_macro(&mut self, token: Token) -> Result<(), ParserError> {
        self.inline_stack
            .push_back(Inline::InlineSpan(InlineSpan::inline_span_from_token(
                token,
            )));
        self.in_inline_span = true;
        Ok(())
    }

    pub(super) fn parse_passthrough_inline_macro(
        &mut self,
        token: Token,
    ) -> Result<(), ParserError> {
        self.open_parse_after_as_text_type = Some(token.token_type());
        Ok(())
    }

    pub(super) fn parse_inline_macro_close(&mut self, token: Token) -> Result<(), ParserError> {
        if let Some(TokenType::PassthroughInlineMacro) = self.open_parse_after_as_text_type {
            self.open_parse_after_as_text_type = None
        } else if let Some(Inline::InlineSpan(span)) = self.inline_stack.back_mut()
            && span.open
            && span.close_open_link()
        {
            // a link macro inside a span (`` `link:x[y]` ``, or a footnote's
            // `footnote:[see link:x[y]]`) closes first; the span stays open
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
            // spans left open inside a footnote are text
            if let Inline::InlineSpan(span) = &mut inline_macro {
                span.unclosed_children_to_text();
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
    pub(super) fn parse_attribute_reference(
        &mut self,
        mut token: Token,
    ) -> Result<(), ParserError> {
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

    pub(super) fn parse_cross_reference(
        &mut self,
        token: Token,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        let Some(text) = InlineRef::xref_text(&token) else {
            self.push_link(Inline::InlineRef(InlineRef::new_xref_from_token(token)));
            self.close_parent_after_push = true;
            return Ok(());
        };
        // an xref with its own text (`<<id,the *A* section>>`): the text is parsed like a link
        // macro's, into the open xref, which then closes
        let (line, col) = (token.line, token.startcol);
        self.push_link(Inline::InlineRef(InlineRef::new_xref_from_token(
            token.clone(),
        )));
        // (the text's tokens start at column 1, so this is one less than the text's column)
        let offset = col + token.lexeme.len() - text.len() - 3;
        for result in Scanner::new(&text) {
            match result {
                Ok(mut text_token) => {
                    if text_token.token_type() == TokenType::Eof {
                        continue;
                    }
                    text_token.update_token_loc_offsets_by(line, offset);
                    self.token_into(text_token, asg)?;
                }
                Err(e) => return Err(ParserError::Scanner(e)),
            }
        }
        self.parse_inline_macro_close(token)
    }

    pub(super) fn parse_code_callout(&mut self, token: Token) -> Result<(), ParserError> {
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

    pub(super) fn parse_text(&mut self, mut token: Token) -> Result<(), ParserError> {
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
            if self.lists.in_description_list() {
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

    pub(super) fn pass_text_through(&mut self, token: Token) -> Result<(), ParserError> {
        if let Some(newline_token) = self.dangling_newline.clone() {
            if self.lists.in_description_list() {
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

    pub(super) fn parse_charref(&mut self, mut token: Token) -> Result<(), ParserError> {
        // an apostrophe in code stays as written (unlike in Asciidoctor, which curls it): a curly
        // quote in code changes the code
        if token.lexeme == "'"
            && let Some(Inline::InlineSpan(span)) = self
                .inline_stack
                .iter()
                .rev()
                .find(|inline| inline.is_open())
            && span.variant == crate::graph::inlines::InlineSpanVariant::Code
        {
            token.literal = Some(token.lexeme.clone());
            return self.parse_text(token);
        }
        let inline_literal = Inline::InlineLiteral(InlineLiteral::new_charref_from_token(&token));
        if let Some(newline_token) = self.dangling_newline.clone() {
            if self.lists.in_description_list() {
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

    pub(super) fn add_text_to_last_inline(&mut self, token: Token) {
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

    /// Makes the last span that is still open text. With `links_too`, a link macro still waiting
    /// for its `]` counts (at the end of a paragraph; in the middle of one, the link may close yet)
    pub(super) fn handle_dangling_spans(&mut self, links_too: bool) {
        // look for the last span in the stack
        if let Some(open_span_idx) = self.inline_stack.iter().rposition(|inline| {
            if links_too {
                inline.is_unclosed()
            } else {
                inline.is_open()
            }
        }) {
            let mut open_span = self.inline_stack.remove(open_span_idx).unwrap();
            // without a closing mark, the span's markup is just text, which goes in front of
            // whatever the span held
            let mut children = open_span.unclosed_as_text();
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
}
