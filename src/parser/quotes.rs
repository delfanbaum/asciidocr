//! Verses and quotes written without delimiters: a paragraph under `[verse]`, and a quoted
//! paragraph (`"text"`, then `-- Author, Title`). A paragraph under `[quote]` needs no more than
//! to be wrapped in a quote when it is complete.

use crate::errors::ParserError;
use crate::graph::{
    asg::Asg,
    blocks::{Block, LeafBlock, LeafBlockForm, LeafBlockName, ParentBlock, ParentBlockName},
    inlines::{Inline, InlineLiteralName},
    metadata::{AttributeType, ElementMetadata},
    substitutions::decode_references,
};
use crate::scanner::{Token, TokenType};

use super::Parser;

impl Parser {
    /// Called with each token, before it is parsed: opens a verse paragraph at the first line of
    /// text under `[verse]`, and closes it at a blank line, or at a line that starts another
    /// block. In between, its lines are text, as in a delimited verse.
    pub(super) fn open_or_close_verse_paragraph(
        &mut self,
        token: &Token,
        asg: &mut Asg,
    ) -> Result<(), ParserError> {
        let token_type = token.token_type();
        if self.in_verse_paragraph {
            let in_list = self
                .lists
                .get(self.block_stack.len().saturating_sub(1))
                .is_some();
            let ends = match token_type {
                TokenType::NewLineChar => self.last_token_type == TokenType::NewLineChar,
                TokenType::Eof => true,
                TokenType::UnorderedListItem
                | TokenType::OrderedListItem
                | TokenType::BlockContinuation => in_list,
                TokenType::BlockAnchor | TokenType::ElementAttributes => true,
                token_type => is_delimiter(token_type),
            };
            if ends {
                self.close_verse_paragraph(asg)?;
            }
        } else if self.starts_verse_paragraph(token_type) {
            self.open_verse_paragraph(token)?;
        }
        Ok(())
    }

    fn starts_verse_paragraph(&self, token_type: TokenType) -> bool {
        self.open_parse_after_as_text_type.is_none()
            && !self.block_title_open
            && self
                .metadata
                .as_ref()
                .is_some_and(|metadata| metadata.declared_type == Some(AttributeType::Verse))
            && !is_delimiter(token_type)
            && !matches!(
                token_type,
                TokenType::NewLineChar
                    | TokenType::Eof
                    | TokenType::Comment
                    | TokenType::Attribute
                    | TokenType::BlockAnchor
                    | TokenType::ElementAttributes
                    | TokenType::BlockLabel
                    | TokenType::BlockContinuation
                    | TokenType::Include
                    | TokenType::StartTag
                    | TokenType::EndTag
            )
    }

    fn open_verse_paragraph(&mut self, token: &Token) -> Result<(), ParserError> {
        let mut verse = LeafBlock::new(
            LeafBlockName::Verse,
            LeafBlockForm::Paragraph,
            None,
            token.locations(),
            vec![],
        );
        if let Some(title) = self.block_title.take() {
            verse.title = title;
        }
        // (a list item the verse is attached to takes no more text)
        self.add_inlines_to_block_stack()?;
        self.dangling_newline = None;
        self.push_block_to_stack(Block::LeafBlock(verse))?;
        self.open_parse_after_as_text_type = Some(TokenType::QuoteVerseBlock);
        self.in_verse_paragraph = true;
        self.force_new_block = false;
        Ok(())
    }

    fn close_verse_paragraph(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
        self.in_verse_paragraph = false;
        self.open_parse_after_as_text_type = None;
        self.force_new_block = false;
        self.add_inlines_to_block_stack()?;
        self.dangling_newline = None;
        match self.block_stack.pop() {
            Some(verse) => self.complete_block(verse, asg),
            None => Err(ParserError::BlockStack(self.line)),
        }
    }
}

/// Whether the token is the delimiter line of a block
fn is_delimiter(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::SidebarBlock
            | TokenType::OpenBlock
            | TokenType::ExampleBlock
            | TokenType::QuoteVerseBlock
            | TokenType::PassthroughBlock
            | TokenType::LiteralBlock
            | TokenType::SourceBlock
            | TokenType::CommentBlock
            | TokenType::Table
    )
}

/// An empty quote written without delimiters, to hold a paragraph
pub(super) fn undelimited_quote(metadata: Option<ElementMetadata>) -> ParentBlock {
    let mut quote = ParentBlock::new(ParentBlockName::Quote, None, "".to_string(), vec![], vec![]);
    quote.metadata = metadata;
    quote
}

/// Takes the quotation marks and the attribution line off a quoted paragraph, as Asciidoctor
/// reads one: the first line starts with `"`, the line before the last ends with `"`, and the
/// last is `-- ` and who said it, optionally followed by a comma and where. Returns the
/// attribution and citation; any other paragraph is left as it is.
pub(super) fn take_quoted_paragraph_attribution(
    inlines: &mut Vec<Inline>,
) -> Option<(String, Option<String>)> {
    // the em dash that starts the last line
    let dash = inlines.iter().rposition(|inline| {
        matches!(inline, Inline::InlineLiteral(literal)
            if matches!(literal.name, InlineLiteralName::Charref) && literal.value() == "&#8212;")
    })?;
    let attribution = text_of(&inlines[dash + 1..]);
    let attribution = attribution.strip_prefix(' ')?.trim();
    if attribution.is_empty() || attribution.contains('\n') {
        return None;
    }
    let quoted = text_of(&inlines[..dash]);
    let quoted = quoted.strip_suffix('\n')?;
    if quoted.len() < 2 || !quoted.starts_with('"') || !quoted.ends_with('"') {
        return None;
    }
    // both marks must be plain text, not part of any formatting
    if !matches!(inlines.first(), Some(Inline::InlineLiteral(_)))
        || !matches!(
            inlines.get(dash.checked_sub(1)?),
            Some(Inline::InlineLiteral(_))
        )
    {
        return None;
    }
    let (attribution, citation) = match attribution.split_once(", ") {
        Some((attribution, citation)) => (attribution, Some(citation.trim())),
        None => (attribution, None),
    };
    let result = (
        decode_references(attribution),
        citation.map(decode_references),
    );

    inlines.truncate(dash);
    // the closing mark and the newline after it (which may be in separate literals)
    let mut remaining = 2;
    while remaining > 0 {
        let Some(Inline::InlineLiteral(last)) = inlines.last_mut() else {
            break;
        };
        remaining -= last.remove_from_end(remaining);
        if last.value().is_empty() {
            inlines.pop();
        }
    }
    if let Some(Inline::InlineLiteral(first)) = inlines.first_mut() {
        first.remove_first_character();
        if first.value().is_empty() {
            inlines.remove(0);
        }
    }
    Some(result)
}

fn text_of(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(Inline::extract_values_to_string)
        .collect()
}
