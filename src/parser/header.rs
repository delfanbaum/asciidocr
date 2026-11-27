//! The document header (title, author and revision lines) and attribute entries

use std::fmt::Debug;

use log::warn;

use crate::errors::ParserError;
use crate::graph::{
    asg::Asg,
    nodes::{Author, Location, Revision},
};
use crate::scanner::{Token, TokenType};

use super::Parser;

/// The lines of the document header, in order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HeaderLine {
    Title,
    /// the line under the title, if it isn't an attribute entry
    Author,
    /// the line under the author line
    Revision,
    /// attribute entries (and anything else) to the end of the header
    Rest,
}

impl Parser {
    pub(super) fn parse_attribute(&mut self, token: Token) -> Result<(), ParserError> {
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
            // unsetting it
            "leveloffset!" | "!leveloffset" => self.level_offset = 0,
            _ => {
                self.document_attributes
                    .insert(key.to_string(), value.to_string());
            }
        }
        Ok(())
    }

    /// Sets the heading level offset: a signed value is relative to the current one, and, as in
    /// Asciidoctor, an empty one resets it. So does one that isn't a number, with a warning
    pub(super) fn parse_level_offset(&mut self, value: &str) {
        let offset = if value.is_empty() {
            0
        } else if let Ok(offset) = value.parse::<i32>() {
            if value.starts_with(['+', '-']) {
                self.level_offset.saturating_add(offset)
            } else {
                offset
            }
        } else {
            warn!(
                "Line {}: Invalid leveloffset \"{value}\"; using 0",
                self.line
            );
            0
        };
        self.level_offset = offset;
    }

    /// Handle document header
    pub(super) fn check_and_move_header(&mut self, asg: &mut Asg) -> Result<(), ParserError> {
        self.end_header_line()?;
        self.add_inlines_to_block_stack()?;
        let mut implied_attributes = vec![];
        if let Some(header) = &mut self.document_header
            && !header.is_empty()
        {
            // without an author line, `:author:` (and `:email:`) entries name the author
            if header.authors.is_none()
                && let Some(author) = self.document_attributes.get("author")
            {
                let line = match self.document_attributes.get("email") {
                    Some(email) => format!("{author} <{email}>"),
                    None => author.clone(),
                };
                header.authors = Some(Author::from_line(&line));
            }
            header.consolidate();
            asg.add_header(header.clone(), self.document_attributes.clone());
            implied_attributes = header.implied_attributes();
        }
        // the attributes the author and revision lines imply (`author`, `revnumber`, ...) work in
        // references, but they aren't the document's own, so the graph doesn't list them (as the
        // AsciiDoc TCK expects)
        self.document_attributes.extend(implied_attributes);

        self.document_header = None;
        Ok(())
    }

    /// The author line and the revision line under the document title are read as they are
    /// written (not as markup), up to the end of their line. Returns whether the token was one of
    /// theirs. Attribute entries extend the header too.
    pub(super) fn read_header_line(&mut self, token: &Token) -> bool {
        let Some(header) = &mut self.document_header else {
            return false;
        };
        match token.token_type() {
            TokenType::NewLineChar | TokenType::Eof => false,
            // an attribute entry: no author or revision line can follow
            TokenType::Attribute => {
                header.location = Location::reconcile(header.location.clone(), token.locations());
                self.header_line = HeaderLine::Rest;
                false
            }
            _ if matches!(self.header_line, HeaderLine::Author | HeaderLine::Revision) => {
                header.location = Location::reconcile(header.location.clone(), token.locations());
                self.header_line_text.push_str(&token.lexeme);
                true
            }
            _ => false,
        }
    }

    /// At the end of a header line: the title is complete, or the author or revision line is
    /// read
    pub(super) fn end_header_line(&mut self) -> Result<(), ParserError> {
        match self.header_line {
            HeaderLine::Title => {
                // (the title's inlines go to the header)
                self.add_inlines_to_block_stack()?;
                self.header_line = HeaderLine::Author;
            }
            HeaderLine::Author | HeaderLine::Revision => {
                let text = std::mem::take(&mut self.header_line_text);
                if let Some(header) = &mut self.document_header
                    && !text.trim().is_empty()
                {
                    if self.header_line == HeaderLine::Author {
                        header.authors = Some(Author::from_line(&text));
                        self.header_line = HeaderLine::Revision;
                    } else {
                        header.revision = Some(Revision::from_line(&text));
                        self.header_line = HeaderLine::Rest;
                    }
                }
            }
            HeaderLine::Rest => {}
        }
        Ok(())
    }
}
