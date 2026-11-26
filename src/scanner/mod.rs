//! The Scanner scans the asciidoc string into Tokens, which is then consumed by the Parser.

pub mod tokens;
use core::str;
use log::{debug, warn};
use tokens::{Token, TokenType};

use crate::errors::ScannerError;

#[derive(Debug)]
/// Scans an asciidoc `&str` into [`Token`]s to be consumed by the Parser.
pub struct Scanner<'a> {
    pub source: &'a str,
    start: usize,
    startcol: usize,
    current: usize,
    line: usize,
    scanning: bool,          // flag to let us know when we're done with a file
    expecting_tag_end: bool, // flag to avoid returning `<--` in lines like `<--end:foo[]-->`
    file_stack: Vec<String>,
}

impl Iterator for Scanner<'_> {
    type Item = Result<Token, ScannerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.is_at_end() {
            self.start = self.current;

            return Some(self.scan_token());
        }
        if self.scanning {
            self.scanning = false;
            return Some(Ok(Token::final_token(self.line, self.file_stack.clone())));
        }
        None
    }
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        Scanner {
            source,
            start: 0,    // beginning of the current lexeme being scanned
            startcol: 1, // because asciidoc spec wants start/end locations
            current: 0,  // the character we're looking at *now*
            line: 1,
            scanning: true,
            expecting_tag_end: false,
            file_stack: vec![],
        }
    }

    pub fn new_with_stack(source: &'a str, file_stack: Vec<String>) -> Self {
        Scanner {
            source,
            start: 0,    // beginning of the current lexeme being scanned
            startcol: 1, // because asciidoc spec wants start/end locations
            current: 0,  // the character we're looking at *now*
            line: 1,
            scanning: true,
            expecting_tag_end: false,
            file_stack,
        }
    }

    fn scan_token(&mut self) -> Result<Token, ScannerError> {
        let c = self.source.as_bytes()[self.current] as char;
        self.current += 1; // this instead of the "advance" function in "Crafting Interpreters"

        // list item markers come first: at the start of a line, `.. ` is a list item and not an
        // ellipsis, and `** ` is a list item and not a strong span
        if matches!(c, ' ' | '\t' | '*' | '-' | '.' | '0'..='9' | 'a'..='z' | 'A'..='Z')
            && let Some((token_type, marker_len)) = self.list_item_marker_at_line_start()
        {
            return self.add_list_item(token_type, marker_len);
        }

        match c {
            '\n' => self.add_token(TokenType::NewLineChar, false, 1),
            '\'' => {
                if self.starts_end_tag_line() {
                    self.add_tag()
                } else if self.starts_repeated_char_line(c, 3) {
                    self.current += 2;
                    self.add_token(TokenType::ThematicBreak, false, 0)
                } else if ['\0', ' ', '\n'].contains(&self.peek_back()) && self.peek() == '`' {
                    self.current += 1;
                    self.add_token(TokenType::OpenSingleQuote, true, 0)
                } else if ["s ", "s\n"].contains(&self.peeks_ahead(2))
                // edge at end case
                    || (self.peeks_ahead(2) == "\0" && self.peek() == 's')
                {
                    self.add_token(TokenType::CharRef, false, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            '<' => {
                if self.starts_end_tag_line() {
                    self.add_tag()
                } else if self.starts_repeated_char_line(c, 3) {
                    self.current += 2;
                    self.add_token(TokenType::PageBreak, false, 0)
                } else if ['-', '='].contains(&self.peek()) {
                    self.current += 1;
                    self.add_token(TokenType::CharRef, false, 0)
                } else if self.peek() == '<' {
                    self.add_cross_reference()
                } else if self.starts_new_line() && self.starts_code_callout_list_item() {
                    self.current += 1; // consume the space char
                    self.add_token(TokenType::CodeCalloutListItem, true, 0)
                } else if self.starts_code_callout() {
                    self.add_token(TokenType::CodeCallout, true, 0)
                // else if ...
                } else {
                    self.add_text_until_next_markup()
                }
            }
            // potential block delimiter chars get treated similarly
            '+' | '*' | '-' | '_' | '/' | '=' | '.' => {
                if self.starts_end_tag_line() {
                    self.add_tag()
                } else if self.starts_new_line() && self.starts_repeated_char_line(c, 4) {
                    self.current += 3; // the remaining repeated chars
                    self.add_token(TokenType::block_from_char(c)?, false, 0)
                } else {
                    match c {
                        '=' => {
                            if self.peek() == '>' {
                                self.current += 1;
                                self.add_token(TokenType::CharRef, false, 0)
                            }
                            // possible heading
                            else if self.starts_new_line() {
                                self.add_heading()
                            } else {
                                self.add_text_until_next_markup()
                            }
                        }
                        '-' => {
                            if self.peek() == '>' {
                                // is it a '->'?
                                self.current += 1;
                                self.add_token(TokenType::CharRef, false, 0)
                            } else if self.starts_text_symbol_replace_emdash() {
                                self.current += 1;
                                self.add_token(TokenType::CharRef, false, 0)
                            } else if self.starts_new_line() && self.starts_repeated_char_line(c, 2)
                            {
                                self.current += 1;
                                // since we consume the newline as a part of the block, add a line
                                self.add_token(TokenType::OpenBlock, false, 0)
                            } else {
                                self.add_text_until_next_markup()
                            }
                        }
                        '*' => self.handle_inline_formatting(
                            c,
                            TokenType::Strong,
                            TokenType::UnconstrainedStrong,
                        ),
                        '/' => {
                            if self.starts_end_tag_line() {
                                self.add_tag()
                            } else if self.starts_new_line() && self.peek() == '/' {
                                if ["/ tag::", "/ end::"].contains(&self.peeks_ahead(7)) {
                                    self.current += 2;
                                    self.add_token(TokenType::Comment, true, 0)
                                } else {
                                    while self.peek() != '\n' && !self.is_at_end() {
                                        self.current += 1;
                                    }
                                    self.add_token(TokenType::Comment, true, 0)
                                }
                            } else {
                                self.add_text_until_next_markup()
                            }
                        }
                        '+' => {
                            if self.starts_new_line() && self.peek() == '\n' {
                                self.add_token(TokenType::BlockContinuation, false, 0)
                            } else if self.peek_back() == ' ' && self.peek() == '\n' {
                                self.add_token(TokenType::LineContinuation, false, 0)
                            } else {
                                self.handle_inline_formatting(
                                    c,
                                    TokenType::Literal,
                                    TokenType::UnconstrainedLiteral,
                                )
                            }
                        }
                        '_' => self.handle_inline_formatting(
                            c,
                            TokenType::Emphasis,
                            TokenType::UnconstrainedEmphasis,
                        ),
                        // ellipsis or block title (ordered list items are found above)
                        '.' => {
                            if self.peeks_ahead(2) == ".." {
                                self.current += 2;
                                self.add_token(TokenType::CharRef, false, 0)
                            } else if self.starts_new_line() {
                                self.add_token(TokenType::BlockLabel, false, 0)
                            } else {
                                self.add_text_until_next_markup()
                            }
                        }

                        _ => self.add_text_until_next_markup(),
                    }
                }
            }

            '[' => {
                // block anchor, role, quote, verse, source, etc. TK add for generic options
                if self.starts_new_line() && self.peek() == '[' {
                    self.add_block_anchor()
                } else if self.starts_attribution_line() {
                    match self.source.as_bytes()[self.start + 1] as char {
                        'N' => self.add_token(TokenType::NotePara, true, 0),
                        'T' => self.add_token(TokenType::TipPara, true, 0),
                        'I' => self.add_token(TokenType::ImportantPara, true, 0),
                        'C' => self.add_token(TokenType::CautionPara, true, 0),
                        'W' => self.add_token(TokenType::WarningPara, true, 0),
                        _ => self.add_token(TokenType::ElementAttributes, true, 0),
                    }
                } else if self.peek() == '.' {
                    self.add_inline_style()
                } else {
                    self.add_text_until_next_markup()
                }
            }

            '`' => {
                if ["' ", "'\n"].contains(&self.peeks_ahead(2))
                    || (self.peeks_ahead(2) == "\0" && self.peek() == '\'')
                {
                    self.current += 1;
                    self.add_token(TokenType::CloseSingleQuote, true, 0)
                } else if ["\" ", "\"\n"].contains(&self.peeks_ahead(2))
                    || (self.peeks_ahead(2) == "\0" && self.peek() == '\"')
                {
                    self.current += 1;
                    self.add_token(TokenType::CloseDoubleQuote, true, 0)
                } else {
                    self.handle_inline_formatting(
                        c,
                        TokenType::Monospace,
                        TokenType::UnconstrainedMonospace,
                    )
                }
            }
            '^' => self.add_token(TokenType::Superscript, false, 0),
            '~' => self.add_token(TokenType::Subscript, false, 0),
            '#' => {
                if self.starts_end_tag_line() {
                    self.add_tag()
                } else {
                    self.handle_inline_formatting(c, TokenType::Mark, TokenType::UnconstrainedMark)
                }
            }
            ':' => {
                if self.starts_new_line() && self.starts_attr() {
                    while self.peek() != '\n' {
                        // TK line continuation
                        self.current += 1
                    }
                    self.add_token(TokenType::Attribute, false, 0)
                } else if let Some(token) = self.description_list_marker(b':', 4) {
                    token
                } else {
                    self.add_text_until_next_markup()
                }
            }
            ';' => {
                if let Some(token) = self.description_list_marker(b';', 2) {
                    token
                } else {
                    self.add_text_until_next_markup()
                }
            }
            '(' => {
                if self.starts_text_symbol_replace_parens() {
                    self.handle_text_symbol_replacement_parens()
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'f' => {
                // guard against indexing out of range
                if self.peeks_ahead(9) == "ootnote:[" {
                    self.current += 9;
                    self.add_token(TokenType::FootnoteMacro, false, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'p' => {
                if self.peeks_ahead(5) == "ass:[" {
                    self.current += 5;
                    self.add_token(TokenType::PassthroughInlineMacro, false, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'h' => {
                if self.peeks_ahead(3) == "ttp" {
                    self.add_link()
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'i' => {
                if self.starts_new_line() && self.peeks_ahead(8) == "nclude::" {
                    self.add_include()
                } else if self.starts_new_line() && self.peeks_ahead(6) == "mage::" {
                    self.add_block_image()
                // double colons after just parse as regular text per asciidoctor implementation
                } else if self.peeks_ahead(5) == "mage:"
                    && self.peeks_ahead(6) != "mage::"
                    && self.peeks_ahead(6) != "mage: "
                {
                    self.add_inline_image()
                } else {
                    self.add_text_until_next_markup()
                }
            }
            't' => {
                if self.peeks_ahead(4) == "ag::" {
                    self.add_tag()
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'e' => {
                if self.peeks_ahead(4) == "nd::" {
                    self.add_tag()
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'N' => {
                if self.peeks_ahead(5) == "OTE: " {
                    self.current += 5;
                    self.add_token(TokenType::NotePara, true, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'T' => {
                if self.peeks_ahead(4) == "IP: " {
                    self.current += 4;
                    self.add_token(TokenType::TipPara, true, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'I' => {
                if self.peeks_ahead(10) == "MPORTANT: " {
                    self.current += 10;
                    self.add_token(TokenType::ImportantPara, true, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'C' => {
                if self.peeks_ahead(8) == "AUTION: " {
                    self.current += 8;
                    self.add_token(TokenType::CautionPara, true, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'W' => {
                if self.peeks_ahead(8) == "ARNING: " {
                    self.current += 8;
                    self.add_token(TokenType::WarningPara, true, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            // Assume that these are macro closes; the parser can always reject it
            ']' => self.add_token(TokenType::InlineMacroClose, true, 0),
            // & chars followed by some number of characters ending with ";" should be considered
            // CharRef tokens... why we need to collect these I'm not entirely sure, but it's in
            // the spec
            '&' => {
                if self.starts_charref() {
                    while self.peek().is_alphanumeric() && !self.is_at_end() {
                        self.current += 1
                    }
                    self.current += 1; // add the ";" at the end
                    self.add_token(TokenType::CharRef, true, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            '{' => {
                if self.starts_attribute_reference() {
                    while (self.peek().is_alphanumeric() || self.peek() == '-') && !self.is_at_end()
                    {
                        self.current += 1
                    }
                    self.current += 1; // add the "}" at the end
                    self.add_token(TokenType::AttributeReference, false, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            '|' => {
                if self.starts_new_line() && self.peeks_ahead(3) == "===" {
                    self.current += 3; // don't consume the newline
                    // check to make sure the next char is a newline or EOF
                    if ['\n', '\0'].contains(&self.peek()) {
                        self.add_token(TokenType::Table, false, 0)
                    } else {
                        self.add_text_until_next_markup()
                    }
                } else if self.starts_new_line() || self.peek_back() == ' ' {
                    // it's either a new line, or if on the same line, must have a space before it
                    self.add_table_cell()
                } else {
                    self.add_text_until_next_markup()
                }
            }
            '\"' => {
                if ['\0', ' ', '\n'].contains(&self.peek_back()) && self.peek() == '`' {
                    self.current += 1;
                    self.add_token(TokenType::OpenDoubleQuote, true, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            ' ' | '%' => {
                if self.starts_end_tag_line() {
                    self.add_tag()
                } else {
                    self.add_text_until_next_markup()
                }
            }
            _ => self.add_text_until_next_markup(),
        }
    }

    fn add_token(
        &mut self,
        token_type: TokenType,
        include_literal: bool,
        advance_line_after: usize,
    ) -> Result<Token, ScannerError> {
        let text = &self.source[self.start..self.current];
        let mut literal = None;
        if include_literal {
            literal = Some(text.to_string())
        }
        let token_start = self.startcol;
        let mut token: Token;

        // save an allocation for the line bumping
        if advance_line_after != 0 {
            let token_line = self.line;
            self.line += advance_line_after;
            self.startcol = 1;
            token = Token {
                token_type,
                lexeme: text.to_string(),
                literal,
                line: token_line,
                startcol: token_start,
                endcol: token_start + text.len() - 1, // to account for the start char
                file_stack: self.file_stack.clone(),
            }
        } else {
            self.startcol = token_start + text.len();
            token = Token {
                token_type,
                lexeme: text.to_string(),
                literal,
                line: self.line,
                startcol: token_start,
                endcol: token_start + text.len() - 1, // to account for the start char
                file_stack: self.file_stack.clone(),
            }
        }
        token.validate();
        debug!("Token {:?} from {}..{}", token, self.start, self.current);
        Ok(token)
    }

    fn add_heading(&mut self) -> Result<Token, ScannerError> {
        while self.peek() == '=' {
            self.current += 1
        }
        self.current += 1; // add the space to the lexeme, but remove from count
        match self.source.as_bytes()[self.start..self.current - 1].len() {
            1 => self.add_token(TokenType::Heading1, false, 0),
            2 => self.add_token(TokenType::Heading2, false, 0),
            3 => self.add_token(TokenType::Heading3, false, 0),
            4 => self.add_token(TokenType::Heading4, false, 0),
            5 => self.add_token(TokenType::Heading5, false, 0),
            _ => {
                warn!("Heading level error at line {}", self.line);
                Err(ScannerError::HeadingLevelError(self.line))
            }
        }
    }

    /// Adds the list item token, whose lexeme is the marker with any indentation before it and
    /// the space after it (the parser trims these)
    fn add_list_item(
        &mut self,
        list_item_token: TokenType,
        marker_len: usize,
    ) -> Result<Token, ScannerError> {
        self.current = self.start + marker_len;
        self.add_token(list_item_token, false, 0)
    }

    /// Checks for a list item marker at the start of a line: one to five `*`, a `-`, one to five
    /// `.`, or a number followed by a `.`, then a space. The marker may be indented. Returns the
    /// token type and the length of the marker, including the indentation and the space.
    fn list_item_marker_at_line_start(&self) -> Option<(TokenType, usize)> {
        if !self.starts_new_line() {
            return None;
        }
        let line = &self.source.as_bytes()[self.start..];
        let indent = line
            .iter()
            .take_while(|b| matches!(b, b' ' | b'\t'))
            .count();
        let rest = &line[indent..];
        let first = *rest.first()?;
        let (token_type, marker_len) = match first {
            b'*' | b'.' => {
                let repeated = rest.iter().take_while(|b| **b == first).count();
                if repeated > 5 {
                    return None;
                }
                let token_type = if first == b'*' {
                    TokenType::UnorderedListItem
                } else {
                    TokenType::OrderedListItem
                };
                (token_type, repeated)
            }
            b'-' => (TokenType::UnorderedListItem, 1),
            b'0'..=b'9' => {
                let digits = rest.iter().take_while(|b| b.is_ascii_digit()).count();
                if rest.get(digits) != Some(&b'.') {
                    return None;
                }
                (TokenType::OrderedListItem, digits + 1)
            }
            // like Asciidoctor: one letter and a dot (`a.`, `B.`), or a roman numeral and a
            // parenthesis (`iv)`, `IX)`)
            b'a'..=b'z' | b'A'..=b'Z' => {
                if rest.get(1) == Some(&b'.') {
                    (TokenType::OrderedListItem, 2)
                } else {
                    let numeral = rest
                        .iter()
                        .take_while(|b| matches!(b, b'i' | b'v' | b'x' | b'I' | b'V' | b'X'))
                        .count();
                    if numeral == 0 || rest.get(numeral) != Some(&b')') {
                        return None;
                    }
                    (TokenType::OrderedListItem, numeral + 1)
                }
            }
            _ => return None,
        };
        if rest.get(marker_len) != Some(&b' ') {
            return None;
        }
        // like Asciidoctor (which strips trailing whitespace first), a marker with no text after
        // it on the line is not a list item
        let has_text = rest[marker_len..]
            .iter()
            .take_while(|b| **b != b'\n')
            .any(|b| !b.is_ascii_whitespace());
        has_text.then_some((token_type, indent + marker_len + 1))
    }

    /// Adds the block image, consuming the target as well as any attributes
    fn add_block_image(&mut self) -> Result<Token, ScannerError> {
        while self.peek() != ']' {
            self.current += 1
        }
        self.current += 1; // consume the ']' char
        self.add_token(TokenType::BlockImageMacro, true, 0)
    }

    /// Adds the block image, consuming the target as well as any attributes
    fn add_inline_image(&mut self) -> Result<Token, ScannerError> {
        while self.peek() != ']' {
            self.current += 1
        }
        self.current += 1; // consume the ']' char
        self.add_token(TokenType::InlineImageMacro, true, 0)
    }

    /// Adds the include image, consuming the target as well as any attributes
    fn add_include(&mut self) -> Result<Token, ScannerError> {
        // a directive that never closes (on its line) is just text
        if !self.closes_on_this_line("]") {
            return self.add_text_until_next_markup();
        }
        while self.peek() != ']' {
            self.current += 1
        }
        self.current += 1; // consume the ']' char
        self.add_token(TokenType::Include, true, 0)
    }

    /// Adds contents into a TableCell token, regardless of other formatting (simpler to re-scan later)
    fn add_table_cell(&mut self) -> Result<Token, ScannerError> {
        while !['\n', '\0', '|'].contains(&self.peek()) {
            self.current += 1
        }
        // if the delimiter is |
        if self.peek() == '|' && self.source.as_bytes()[self.current - 1] as char != ' ' {
            self.current += 1;
            return self.add_table_cell();
        }
        self.add_token(TokenType::TableCell, true, 0)
    }

    /// Adds the link, consuming the target but not any attributes. Like Asciidoctor, a bare URL
    /// runs to whitespace, a bracket or a `<`, less the punctuation at its end that belongs to
    /// the sentence (`,`, `.`, `?`, `!`, `)`, and here `:` and `;` too) or closes formatting
    /// around it (`*`, `_`, `` ` ``, `#`).
    fn add_link(&mut self) -> Result<Token, ScannerError> {
        let url_length = self.source[self.current..]
            .find(|c: char| c.is_whitespace() || matches!(c, '[' | ']' | '<'))
            .unwrap_or(self.source.len() - self.current);
        self.current += url_length;
        // it's an attribute-styled link
        if self.peek() == '[' {
            self.current += 1; // consume the '[' char
            return self.add_token(TokenType::LinkMacro, true, 0);
        }
        let trailing = self.source[self.start..self.current]
            .chars()
            .rev()
            .take_while(|c| {
                matches!(
                    c,
                    ',' | '.' | '?' | '!' | ')' | ':' | ';' | '*' | '_' | '`' | '#'
                )
            })
            .count();
        // (the trailing characters are ASCII, so their count is their length in bytes)
        self.current -= trailing.min(self.current - self.start - 1);
        self.add_token(TokenType::Hyperlink, true, 0)
    }

    fn add_inline_style(&mut self) -> Result<Token, ScannerError> {
        while self.peek() != ']' {
            self.current += 1
        }
        self.current += 1; // consume the ']' char
        self.add_token(TokenType::InlineStyle, true, 0)
    }

    /// Whether the given closing markup shows up before the end of the current line
    fn closes_on_this_line(&self, closing: &str) -> bool {
        self.source[self.current..]
            .split('\n')
            .next()
            .is_some_and(|rest_of_line| rest_of_line.contains(closing))
    }

    // adds cross references, e.g., "<<some_id>>"
    fn add_cross_reference(&mut self) -> Result<Token, ScannerError> {
        // a cross reference that never closes (on its line) is just text
        if !self.closes_on_this_line(">>") {
            return self.add_text_until_next_markup();
        }
        while self.peeks_ahead(2) != ">>" && !self.is_at_end() {
            self.current += 1
        }
        self.current += 2; // consume the '>>' chars
        self.add_token(TokenType::CrossReference, true, 0)
    }

    // adds block anchors, e.g., "\n[[some_block_id]]\n"
    fn add_block_anchor(&mut self) -> Result<Token, ScannerError> {
        // an anchor that never closes (on its line) is just text
        if !self.closes_on_this_line("]]") {
            return self.add_text_until_next_markup();
        }
        while self.peeks_ahead(3) != "]]\n" && !self.is_at_end() {
            self.current += 1
        }
        self.current += 2; // consume the ']]' chars, NOT newline
        self.add_token(TokenType::BlockAnchor, true, 0)
    }

    /// A description list marker at the start of the token: a run of two to four colons, or of
    /// two semicolons, followed by a space, a newline or the end of the file. Like Asciidoctor, a
    /// longer run ends with the marker, and the colons (or semicolons) before it are the end of
    /// the term, so they are text. Unlike in Asciidoctor, a run after a space is not a marker
    /// (`Term :: def` is text).
    fn description_list_marker(
        &mut self,
        mark: u8,
        max_len: usize,
    ) -> Option<Result<Token, ScannerError>> {
        let bytes = self.source.as_bytes();
        // where the run started: before this token, if the start of the run was split off as text
        let run_start = self.start
            - bytes[..self.start]
                .iter()
                .rev()
                .take_while(|b| **b == mark)
                .count();
        if run_start > 0 && matches!(bytes[run_start - 1], b' ' | b'\t') {
            return None;
        }
        let run = bytes[self.start..]
            .iter()
            .take_while(|b| **b == mark)
            .count();
        let after = bytes.get(self.start + run);
        if run < 2 || !matches!(after, None | Some(b' ') | Some(b'\n')) {
            return None;
        }
        if run > max_len {
            self.current = self.start + run - max_len;
            return Some(self.add_token(TokenType::Text, true, 0));
        }
        // the space after the marker is part of it, but a newline is its own token, so that the
        // line is counted
        self.current = self.start + run + usize::from(after == Some(&b' '));
        Some(self.add_token(TokenType::DescriptionListMarker, false, 0))
    }

    fn add_text_until_next_markup(&mut self) -> Result<Token, ScannerError> {
        // "inline" chars that could be markup; the newline condition prevents
        // capturing significant block markup chars
        // Chars: newline, bold, italic, code, super, subscript, footnote, pass, link, end inline macro, definition list marker, highlighted, inline admonition initial chars, inline images
        while ![
            '\n', '*', '_', '`', '^', '~', 'f', 'p', 'h', ']', '[', ':', '#', 'N', 'T', 'I', 'C',
            'W', '&', '{', '+', 'i', '<', '\'', '\"', '-', 't', 'e', ';',
        ]
        .contains(&self.peek())
            && !self.is_at_end()
        {
            self.current += 1;
        }
        self.add_token(TokenType::Text, true, 0)
    }

    fn add_tag(&mut self) -> Result<Token, ScannerError> {
        let tag_type: TokenType;
        if self.expecting_tag_end {
            while self.peek() != 'e' {
                self.current += 1;
            }
            self.start = self.current;
            self.startcol = self.start;
        }
        while ![']'].contains(&self.peek()) && !self.is_at_end() {
            self.current += 1;
        }
        self.current += 1; // consume the ']' char

        if (self.source[self.start..self.current]).contains("tag") {
            tag_type = TokenType::StartTag;
            self.expecting_tag_end = true;
        } else if (self.source[self.start..self.current]).contains("end") {
            tag_type = TokenType::EndTag;
            self.expecting_tag_end = false;
        } else {
            return Err(ScannerError::TagError(
                self.line,
                self.source[self.start..self.current].to_string(),
            ));
        }
        let result = self.add_token(tag_type, true, 1);
        // move to the end of the line
        while self.peek() != '\n' && !self.is_at_end() {
            self.current += 1
        }
        if self.peek() == '\n' {
            self.current += 1
        }
        result
    }

    fn starts_new_line(&self) -> bool {
        self.start == 0 || self.source.as_bytes()[self.start - 1] == b'\n'
    }

    fn starts_repeated_char_line(&self, c: char, delimiter_len: usize) -> bool {
        let mut expected_block = String::from(c).repeat(delimiter_len - 1);
        expected_block.push('\n');

        self.current + delimiter_len <= self.source.len()
            && self.peeks_ahead(delimiter_len) == expected_block
    }

    /// Checks for document attribute lines, e.g., ":foo: bar" or ":foo:"
    fn starts_attr(&mut self) -> bool {
        let current_placeholder = self.current;
        while ![' ', '\n', ':'].contains(&self.peek()) && !self.is_at_end() {
            self.current += 1
        }
        let check = self.source.as_bytes()[self.current] as char == ':';
        self.current = current_placeholder;
        check
    }

    fn starts_end_tag_line(&mut self) -> bool {
        self.expecting_tag_end && self.peek_line().contains("end::")
    }

    /// Checks for document attribute references, e.g., {my-thing}
    /// According to the asciidoctor docs, they must be:
    /// * be at least one character long,
    /// * begin with a word character (A-Z, a-z, 0-9, or _), and
    /// * only contain word characters and hyphens.
    fn starts_attribute_reference(&mut self) -> bool {
        let current_placeholder = self.current;
        while (self.peek().is_alphanumeric() || self.peek() == '-') && !self.is_at_end() {
            self.current += 1
        }
        let check = self.source.as_bytes()[self.current] as char == '}';
        self.current = current_placeholder;
        check
    }

    /// Checks for CharRefs, i.e., &plus; type things
    fn starts_charref(&mut self) -> bool {
        let current_placeholder = self.current;
        while self.peek().is_alphanumeric() && !self.is_at_end() {
            self.current += 1
        }
        let check = self.source.as_bytes()[self.current] as char == ';';
        self.current = current_placeholder;
        check
    }

    /// Handles inline formatting, constrained or not
    fn handle_inline_formatting(
        &mut self,
        c: char,
        constrained: TokenType,
        unconstrained: TokenType,
    ) -> Result<Token, ScannerError> {
        let inline_markup_chars = ['*', '_', '`', '+', '^', '~', '#'];
        let mut end_of_inline_markers = vec![
            ' ', '\0', '.', ',', ';', ':', '\n', ')', '"', '!', '?', '\'', ']', '…', '“', '”', '‘',
            '’',
        ];
        let mut beginning_of_inline_markers = vec![' ', '\n', '\0', ']', '(', '"', '['];
        end_of_inline_markers.extend_from_slice(&inline_markup_chars);
        beginning_of_inline_markers.extend_from_slice(&inline_markup_chars);
        // guard clause against dangling markup
        if self.peek() == ' ' && self.peek_back() == ' ' {
            self.add_text_until_next_markup()
        } else if self.peek() == c {
            // the next character is the same
            // we've got an unconstrained (i.e., "**foo**bar") version
            self.current += 1;
            self.add_token(unconstrained, false, 0)
        }
        // we're at the end of a span, or are butted up against another inline marker
        else if end_of_inline_markers.contains(&self.peek()) ||
        // or we're at the beginning, or butted up against another inline marker
        beginning_of_inline_markers.contains(&self.peek_back()) && self.peek() != c
        {
            self.add_token(constrained, false, 0)
        } else {
            self.add_text_until_next_markup()
        }
    }

    fn handle_text_symbol_replacement_parens(&mut self) -> Result<Token, ScannerError> {
        while self.peek() != ')' {
            self.current += 1
        }
        self.current += 1;
        self.add_token(TokenType::CharRef, true, 0)
    }

    /// Checks for lines such as [quote], [verse, Mary Oliver], [source, python], etc.
    fn starts_attribution_line(&mut self) -> bool {
        let current_placeholder = self.current;
        while self.peek() != '\n' && !self.is_at_end() {
            self.current += 1;
        }
        // handle "only on new block save after comment" in the parser
        if self.starts_new_line() && self.source.as_bytes()[self.current - 1] as char == ']' {
            // i.e., the end of an attribute list line
            true
        } else {
            self.current = current_placeholder;
            false
        }
    }

    fn starts_code_callout_list_item(&mut self) -> bool {
        while self.peek() != '>' {
            if self.peek().is_ascii_digit() {
                self.current += 1;
            } else {
                return false;
            }
        }
        self.current += 1;
        self.peek() == ' ' // next must be a space
    }

    fn starts_code_callout(&mut self) -> bool {
        while self.peek() != '>' {
            if self.peek().is_ascii_digit() {
                self.current += 1;
            } else {
                return false;
            }
        }
        self.current += 1;
        self.peek() == '\n' // callouts MUST be last chars in line
    }

    fn starts_text_symbol_replace_parens(&mut self) -> bool {
        ["C)", "R)"].contains(&self.peeks_ahead(2)) || self.peeks_ahead(3) == "TM)"
    }

    fn starts_text_symbol_replace_emdash(&mut self) -> bool {
        let after_char = self.peeks_ahead(2).chars().last().unwrap();
        self.peek() == '-' && (after_char.is_alphanumeric() || [' ', '\0'].contains(&after_char))
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn peek(&self) -> char {
        if self.is_at_end() || !self.source.is_char_boundary(self.current) {
            return '\0';
        }
        self.source.as_bytes()[self.current] as char
    }

    fn peek_back(&self) -> char {
        if self.start == 0 || !self.source.is_char_boundary(self.start - 1) {
            return '\0';
        }
        self.source.as_bytes()[self.start - 1] as char
    }

    fn peeks_ahead(&self, count: usize) -> &str {
        if self.is_at_end()
            || self.current + count > self.source.len()
            || !self.source.is_char_boundary(self.current + count)
        {
            return "\0";
        }
        &self.source[self.current..self.current + count]
    }

    fn peek_line(&self) -> &str {
        let mut count = 1;
        while !['\n', '\0'].contains(&self.peeks_ahead(count).chars().last().unwrap()) {
            count += 1
        }
        self.peeks_ahead(count - 1)
    }
}

#[cfg(test)]
mod tests {
    use std::char;

    use crate::scanner::tokens::TokenType;
    use rstest::rstest;

    use super::*;

    fn scan_and_assert_eq(markup: &str, expected_tokens: Vec<Token>) {
        let s = Scanner::new(markup);
        let mut scanned_tokens: Vec<Token> = s
            .into_iter()
            .filter_map(|result| result.as_ref().ok().cloned())
            .collect();
        let _ = scanned_tokens.pop();
        assert_eq!(scanned_tokens, expected_tokens);
    }

    fn newline_token_at(line: usize, col: usize) -> Token {
        Token::new_default(
            TokenType::NewLineChar,
            "\n".to_string(),
            None,
            line,
            col,
            col,
        )
    }

    #[test]
    fn newline() {
        let markup = "\n".to_string();
        let expected_tokens = vec![newline_token_at(1, 1)];
        scan_and_assert_eq(&markup, expected_tokens)
    }

    #[rstest]
    #[case("++++\n", TokenType::PassthroughBlock)]
    #[case("****\n", TokenType::SidebarBlock)]
    #[case("----\n", TokenType::SourceBlock)]
    #[case("____\n", TokenType::QuoteVerseBlock)]
    #[case("____\n", TokenType::QuoteVerseBlock)]
    #[case("====\n", TokenType::ExampleBlock)]
    #[case("....\n", TokenType::LiteralBlock)]
    fn fenced_block_delimiter_start(#[case] markup: &str, #[case] expected_token: TokenType) {
        let expected_tokens = vec![
            Token::new_default(
                expected_token,
                markup.to_string()[..4].to_string(),
                None,
                1,
                1,
                4,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 5, 5),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[rstest]
    #[case("\n\n++++\n", TokenType::PassthroughBlock)]
    #[case("\n\n****\n", TokenType::SidebarBlock)]
    #[case("\n\n----\n", TokenType::SourceBlock)]
    #[case("\n\n____\n", TokenType::QuoteVerseBlock)]
    #[case("\n\n////\n", TokenType::CommentBlock)]
    #[case("\n\n====\n", TokenType::ExampleBlock)]
    #[case("\n\n....\n", TokenType::LiteralBlock)]
    fn fenced_block_delimiter_new_block(#[case] markup: &str, #[case] expected_token: TokenType) {
        let expected_tokens = vec![
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 2, 1, 1),
            Token::new_default(
                expected_token,
                markup.to_string()[2..6].to_string(),
                None,
                3,
                1,
                4,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 3, 5, 5),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn open_block_beginning() {
        let markup = "--\n";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::OpenBlock,
                markup.to_string()[..2].to_string(),
                None,
                1,
                1,
                2,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 3, 3),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn open_block_new_block() {
        let markup = "\n\n--\n";
        let expected_tokens = vec![
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 2, 1, 1),
            Token::new_default(TokenType::OpenBlock, "--".to_string(), None, 3, 1, 2),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 3, 3, 3),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[rstest]
    #[case("* Foo\n".to_string(), TokenType::UnorderedListItem)]
    #[case(". Foo\n".to_string(), TokenType::OrderedListItem)]
    fn single_list_items(#[case] markup: String, #[case] expected_token: TokenType) {
        let mut delimiter = markup
            .clone()
            .split_whitespace()
            .next()
            .unwrap()
            .to_string();
        delimiter.push(' ');
        let expected_tokens = vec![
            Token::new_default(expected_token, delimiter, None, 1, 1, 2),
            Token::new_default(
                TokenType::Text,
                "Foo".to_string(),
                Some("Foo".to_string()),
                1,
                3,
                5,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 6, 6),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case::unordered("* Foo\n* Bar".to_string(), TokenType::UnorderedListItem)]
    #[case::ordered(". Foo\n. Bar".to_string(), TokenType::OrderedListItem)]
    fn multiple_list_items(#[case] markup: String, #[case] expected_token: TokenType) {
        let mut delimiter = markup
            .clone()
            .split_whitespace()
            .next()
            .unwrap()
            .to_string();
        delimiter.push(' ');
        let expected_tokens = vec![
            Token::new_default(expected_token, delimiter.clone(), None, 1, 1, 2),
            Token::new_default(
                TokenType::Text,
                "Foo".to_string(),
                Some("Foo".to_string()),
                1,
                3,
                5,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 6, 6),
            Token::new_default(expected_token, delimiter, None, 2, 1, 2),
            Token::new_default(
                TokenType::Text,
                "Bar".to_string(),
                Some("Bar".to_string()),
                2,
                3,
                5,
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    /// The lexeme keeps any indentation and the space after the marker
    #[rstest]
    #[case::double_star("** Foo\n", "** ", TokenType::UnorderedListItem)]
    #[case::five_stars("***** Foo\n", "***** ", TokenType::UnorderedListItem)]
    #[case::dash("- Foo\n", "- ", TokenType::UnorderedListItem)]
    #[case::double_dot(".. Foo\n", ".. ", TokenType::OrderedListItem)]
    #[case::triple_dot("... Foo\n", "... ", TokenType::OrderedListItem)]
    #[case::numbered("12. Foo\n", "12. ", TokenType::OrderedListItem)]
    #[case::indented("  * Foo\n", "  * ", TokenType::UnorderedListItem)]
    #[case::tab_indented("\t. Foo\n", "\t. ", TokenType::OrderedListItem)]
    #[case::lower_alpha("a. Foo\n", "a. ", TokenType::OrderedListItem)]
    #[case::upper_alpha("B. Foo\n", "B. ", TokenType::OrderedListItem)]
    #[case::lower_roman("iv) Foo\n", "iv) ", TokenType::OrderedListItem)]
    #[case::upper_roman("IX) Foo\n", "IX) ", TokenType::OrderedListItem)]
    fn list_item_markers(
        #[case] markup: &str,
        #[case] marker: &str,
        #[case] expected_token: TokenType,
    ) {
        let text_start = marker.len() + 1;
        let expected_tokens = vec![
            Token::new_default(expected_token, marker.to_string(), None, 1, 1, marker.len()),
            Token::new_default(
                TokenType::Text,
                "Foo".to_string(),
                Some("Foo".to_string()),
                1,
                text_start,
                text_start + 2,
            ),
            Token::new_default(
                TokenType::NewLineChar,
                "\n".to_string(),
                None,
                1,
                text_start + 3,
                text_start + 3,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    /// Markup that looks like a list item marker, but isn't one
    #[rstest]
    #[case::six_stars("****** Foo\n")]
    #[case::six_dots("...... Foo\n")]
    #[case::no_space_after_star("*Foo*\n")]
    #[case::no_space_after_dash("-Foo\n")]
    #[case::double_dash("-- Foo\n")]
    #[case::number_without_dot("12 Foo\n")]
    #[case::number_without_space("1.5 Foo\n")]
    #[case::ellipsis("...Foo\n")]
    #[case::mid_line("Foo * Bar\n")]
    #[case::no_text_after_marker("* \n")]
    #[case::only_whitespace_after_marker(" *  \t\nFoo\n")]
    #[case::two_letters("ab. Foo\n")]
    #[case::letter_without_space("a.Foo\n")]
    #[case::letter_with_parenthesis("a) Foo\n")]
    #[case::number_with_parenthesis("1) Foo\n")]
    #[case::not_roman("q) Foo\n")]
    #[case::roman_with_dot("ii. Foo\n")]
    fn not_list_item_markers(#[case] markup: &str) {
        assert!(Scanner::new(markup).all(|token| !matches!(
            token.unwrap().token_type(),
            TokenType::UnorderedListItem | TokenType::OrderedListItem
        )));
    }

    #[test]
    fn block_introduction() {
        // skip the "i"
        let markup = ".Baz\n";
        let title = "Baz".to_string();
        let expected_tokens = vec![
            Token::new_default(TokenType::BlockLabel, ".".to_string(), None, 1, 1, 1),
            Token::new_default(TokenType::Text, title.clone(), Some(title), 1, 2, 4),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 5, 5),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[rstest]
    #[case("\n\n= Foo\n".to_string(), TokenType::Heading1, 1)]
    #[case("\n\n== Foo\n".to_string(), TokenType::Heading2, 2)]
    #[case("\n\n=== Foo\n".to_string(), TokenType::Heading3, 3)]
    #[case("\n\n==== Foo\n".to_string(), TokenType::Heading4, 4)]
    #[case("\n\n===== Foo\n".to_string(), TokenType::Heading5, 5)]
    fn headings_after_block(
        #[case] markup: String,
        #[case] expected_token: TokenType,
        #[case] heading_level: usize,
    ) {
        let mut lexeme = "=".to_string().repeat(heading_level);
        lexeme.push(' ');
        let expected_tokens = vec![
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 2, 1, 1),
            Token::new_default(expected_token, lexeme.clone(), None, 3, 1, lexeme.len()),
            Token::new_default(
                TokenType::Text,
                "Foo".to_string(),
                Some("Foo".to_string()),
                3,
                lexeme.len() + 1,
                lexeme.len() + 3,
            ),
            Token::new_default(
                TokenType::NewLineChar,
                "\n".to_string(),
                None,
                3,
                lexeme.len() + 4,
                lexeme.len() + 4,
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case("\n\n'''\n".to_string(), TokenType::ThematicBreak)]
    #[case("\n\n<<<\n".to_string(), TokenType::PageBreak)]
    fn breaks(#[case] markup: String, #[case] expected_token: TokenType) {
        // these should always be after a block, and the 'start' case is tested elsewhere
        let expected_tokens = vec![
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 2, 1, 1),
            Token::new_default(
                expected_token,
                markup.clone()[2..5].to_string(),
                None,
                3,
                1,
                3,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 3, 4, 4),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn comments() {
        let comment_line = "// Some text or other".to_string();
        let markup = "\n".to_string() + &comment_line + "\n";
        let expected_tokens = vec![
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new_default(
                TokenType::Comment,
                comment_line.clone(),
                Some(comment_line.clone()),
                2,
                1,
                comment_line.len(),
            ),
            Token::new_default(
                TokenType::NewLineChar,
                "\n".to_string(),
                None,
                2,
                comment_line.len() + 1,
                comment_line.len() + 1,
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case::quote("[quote]\n", TokenType::ElementAttributes)]
    #[case::quote("[#class.role]\n", TokenType::ElementAttributes)]
    #[case("[quote, Georges Perec]\n", TokenType::ElementAttributes)]
    #[case("[verse]\n", TokenType::ElementAttributes)]
    #[case(
        "[verse, Audre Lorde, A Litany for Survival]\n",
        TokenType::ElementAttributes
    )]
    #[case("[source]\n", TokenType::ElementAttributes)]
    #[case::role("[role=\"foo\"]\n", TokenType::ElementAttributes)]
    #[case::role_dot("[.foo]\n", TokenType::ElementAttributes)]
    #[case("[NOTE]\n", TokenType::NotePara)]
    #[case("[TIP]\n", TokenType::TipPara)]
    #[case("[IMPORTANT]\n", TokenType::ImportantPara)]
    #[case("[CAUTION]\n", TokenType::CautionPara)]
    #[case("[WARNING]\n", TokenType::WarningPara)]
    fn attribute_lines(#[case] markup: &str, #[case] expected_token: TokenType) {
        let markup_len = markup[..markup.len() - 1].len();
        let expected_tokens = vec![
            Token::new_default(
                expected_token,
                markup[..markup.len() - 1].to_string(),
                Some(markup[..markup.len() - 1].to_string()),
                1,
                1,
                markup_len,
            ),
            Token::new_default(
                TokenType::NewLineChar,
                "\n".to_string(),
                None,
                1,
                markup.len(),
                markup.len(),
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn block_continuation() {
        let markup = "* Foo\n+\nBar".to_string();
        let expected_tokens = vec![
            Token::new_default(
                TokenType::UnorderedListItem,
                "* ".to_string(),
                None,
                1,
                1,
                2,
            ),
            Token::new_default(
                TokenType::Text,
                "Foo".to_string(),
                Some("Foo".to_string()),
                1,
                3,
                5,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 6, 6),
            Token::new_default(TokenType::BlockContinuation, "+".to_string(), None, 2, 1, 1),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 2, 2, 2),
            Token::new_default(
                TokenType::Text,
                "Bar".to_string(),
                Some("Bar".to_string()),
                3,
                1,
                3,
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn line_continuation() {
        let markup = "Foo +\nBar".to_string();
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Foo ".to_string(),
                Some("Foo ".to_string()),
                1,
                1,
                4,
            ),
            Token::new_default(TokenType::LineContinuation, "+".to_string(), None, 1, 5, 5),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 6, 6),
            Token::new_default(
                TokenType::Text,
                "Bar".to_string(),
                Some("Bar".to_string()),
                2,
                1,
                3,
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case('*', TokenType::Strong)]
    #[case('_', TokenType::Emphasis)]
    #[case('`', TokenType::Monospace)]
    #[case('+', TokenType::Literal)]
    #[case('^', TokenType::Superscript)]
    #[case('~', TokenType::Subscript)]
    #[case('#', TokenType::Mark)]
    fn inline_formatting(#[case] markup_char: char, #[case] expected_token: TokenType) {
        let markup = format!("Somx {}bar{} bar.", markup_char, markup_char);
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Somx ".to_string(),
                Some("Somx ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(expected_token, markup_char.to_string(), None, 1, 6, 6),
            Token::new_default(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                7,
                9,
            ),
            Token::new_default(expected_token, markup_char.to_string(), None, 1, 10, 10),
            Token::new_default(
                TokenType::Text,
                " bar.".to_string(),
                Some(" bar.".to_string()),
                1,
                11,
                markup.len(),
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case::strong(String::from("**"), TokenType::UnconstrainedStrong)]
    #[case::emphasis(String::from("__"), TokenType::UnconstrainedEmphasis)]
    #[case::monospace(String::from("``"), TokenType::UnconstrainedMonospace)]
    #[case::mark(String::from("##"), TokenType::UnconstrainedMark)]
    #[case::mark(String::from("++"), TokenType::UnconstrainedLiteral)]
    fn inline_formatting_doubles(#[case] markup_str: String, #[case] expected_token: TokenType) {
        let markup = format!("Somx{}bar{}bar.", markup_str, markup_str);
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Somx".to_string(),
                Some("Somx".to_string()),
                1,
                1,
                4,
            ),
            Token::new_default(expected_token, markup_str.clone(), None, 1, 5, 6),
            Token::new_default(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                7,
                9,
            ),
            Token::new_default(expected_token, markup_str, None, 1, 10, 11),
            Token::new_default(
                TokenType::Text,
                "bar.".to_string(),
                Some("bar.".to_string()),
                1,
                12,
                15,
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case('*', TokenType::Strong)]
    #[case('`', TokenType::Monospace)]
    #[case('+', TokenType::Literal)]
    #[case('^', TokenType::Superscript)]
    #[case('~', TokenType::Subscript)]
    #[case('#', TokenType::Mark)]
    fn inline_formatting_by_other(#[case] markup_char: char, #[case] expected_token: TokenType) {
        let markup = format!("Somx {}_bar_{} bar.", markup_char, markup_char);
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Somx ".to_string(),
                Some("Somx ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(expected_token, markup_char.to_string(), None, 1, 6, 6),
            Token::new_default(TokenType::Emphasis, "_".to_string(), None, 1, 7, 7),
            Token::new_default(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                8,
                10,
            ),
            Token::new_default(TokenType::Emphasis, "_".to_string(), None, 1, 11, 11),
            Token::new_default(expected_token, markup_char.to_string(), None, 1, 12, 12),
            Token::new_default(
                TokenType::Text,
                " bar.".to_string(),
                Some(" bar.".to_string()),
                1,
                13,
                markup.len(),
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case('*')]
    #[case('_')]
    #[case('`')]
    #[case('+')]
    #[case('#')]
    // note that ^ and ~ transformations are handled better by the parser
    fn inline_formatting_by_self_is_text(#[case] markup_char: char) {
        let markup = format!(" {} ", markup_char);
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                " ".to_string(),
                Some(" ".to_string()),
                1,
                1,
                1,
            ),
            Token::new_default(
                TokenType::Text,
                format!("{} ", markup_char),
                Some(format!("{} ", markup_char)),
                1,
                2,
                3,
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case("footnote:[", TokenType::FootnoteMacro)]
    #[case("pass:[", TokenType::PassthroughInlineMacro)]
    fn inline_macros(#[case] markup_check: &str, #[case] expected_token: TokenType) {
        let markup = format!("Somx {}bar]", &markup_check);
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Somx ".to_string(),
                Some("Somx ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(
                expected_token,
                markup_check.to_string(),
                None,
                1,
                6,
                6 + markup_check.len() - 1,
            ),
            Token::new_default(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                6 + markup_check.len(),
                6 + markup_check.len() + 2,
            ),
            Token::new_default(
                TokenType::InlineMacroClose,
                "]".to_string(),
                Some("]".to_string()),
                1,
                markup.len(),
                markup.len(),
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case("NOTE", TokenType::NotePara)]
    #[case("TIP", TokenType::TipPara)]
    #[case("IMPORTANT", TokenType::ImportantPara)]
    #[case("CAUTION", TokenType::CautionPara)]
    #[case("WARNING", TokenType::WarningPara)]
    fn inline_admonitions(#[case] markup_check: &str, #[case] expected_token: TokenType) {
        let markup = format!("{}: bar.", markup_check);
        let expected_tokens = vec![
            Token::new_default(
                expected_token,
                format!("{}: ", markup_check),
                Some(format!("{}: ", markup_check)),
                1,
                1,
                markup_check.len() + 2, // account for space
            ),
            Token::new_default(
                TokenType::Text,
                "bar.".to_string(),
                Some("bar.".to_string()),
                1,
                markup_check.len() + 3,
                markup_check.len() + 6,
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn description_list_mark_space() {
        // make it "DS" instead of "Description" to avoid the "i" delimiting the text
        let markup = "Txrm:: DS";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Txrm".to_string(),
                Some("Txrm".to_string()),
                1,
                1,
                4,
            ),
            Token::new_default(
                TokenType::DescriptionListMarker,
                ":: ".to_string(),
                None,
                1,
                5,
                7,
            ),
            Token::new_default(
                TokenType::Text,
                "DS".to_string(),
                Some("DS".to_string()),
                1,
                8,
                9,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn description_list_mark_newline() {
        // make it "DS" instead of "Description" to avoid the "i" delimiting the text
        let markup = "Txrm::\nDS";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Txrm".to_string(),
                Some("Txrm".to_string()),
                1,
                1,
                4,
            ),
            // the newline is its own token, so that the lines after it are counted
            Token::new_default(
                TokenType::DescriptionListMarker,
                "::".to_string(),
                None,
                1,
                5,
                6,
            ),
            newline_token_at(1, 7),
            Token::new_default(
                TokenType::Text,
                "DS".to_string(),
                Some("DS".to_string()),
                2,
                1,
                2,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    /// The text before the marker, and the marker's lexeme. Like Asciidoctor, a run of more
    /// colons (or semicolons) than a marker has ends with the marker, and the rest is the term's
    #[rstest]
    #[case::double_colon("Txrm:: DS", "Txrm", ":: ")]
    #[case::triple_colon("Txrm::: DS", "Txrm", "::: ")]
    #[case::quadruple_colon("Txrm:::: DS", "Txrm", ":::: ")]
    #[case::double_semicolon("Txrm;; DS", "Txrm", ";; ")]
    #[case::five_colons("Txrm::::: DS", "Txrm:", ":::: ")]
    #[case::three_semicolons("Txrm;;; DS", "Txrm;", ";; ")]
    #[case::end_of_line("Txrm:::\nDS", "Txrm", ":::")]
    fn description_list_markers(#[case] markup: &str, #[case] term: &str, #[case] marker: &str) {
        let tokens: Vec<Token> = Scanner::new(markup).filter_map(|t| t.ok()).collect();
        let marker_idx = tokens
            .iter()
            .position(|t| t.token_type() == TokenType::DescriptionListMarker)
            .unwrap_or_else(|| panic!("No marker in {tokens:?}"));
        let text: String = tokens[..marker_idx]
            .iter()
            .map(|t| t.lexeme.clone())
            .collect();
        assert_eq!(text, term);
        assert_eq!(tokens[marker_idx].lexeme, marker);
    }

    #[rstest]
    #[case::colons_in_text("a::b::c\n")]
    #[case::semicolons_in_text("a;;b\n")]
    #[case::space_before_semicolons("Txrm ;; bar\n")]
    #[case::space_before_three_colons("Txrm ::: bar\n")]
    #[case::space_before_four_colons("Txrm :::: bar\n")]
    fn not_description_list_markers(#[case] markup: &str) {
        assert!(
            Scanner::new(markup)
                .filter_map(|t| t.ok())
                .all(|t| t.token_type() != TokenType::DescriptionListMarker)
        );
    }

    #[test]
    fn description_list_mark_not_if_space_before() {
        let markup = "Txrm :: bar";

        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Txrm ".to_string(),
                Some("Txrm ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(
                TokenType::Text,
                ":".to_string(),
                Some(":".to_string()),
                1,
                6,
                6,
            ),
            Token::new_default(
                TokenType::Text,
                ": bar".to_string(),
                Some(": bar".to_string()),
                1,
                7,
                11,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn link_with_text() {
        let markup = "Somx http://example.com[bar]";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Somx ".to_string(),
                Some("Somx ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(
                TokenType::LinkMacro,
                "http://example.com[".to_string(),
                Some("http://example.com[".to_string()),
                1,
                6,
                24,
            ),
            Token::new_default(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                25,
                27,
            ),
            Token::new_default(
                TokenType::InlineMacroClose,
                "]".to_string(),
                Some("]".to_string()),
                1,
                28,
                28,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn link_no_text() {
        let markup = "Somx http://example.com[]";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Somx ".to_string(),
                Some("Somx ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(
                TokenType::LinkMacro,
                "http://example.com[".to_string(),
                Some("http://example.com[".to_string()),
                1,
                6,
                24,
            ),
            Token::new_default(
                TokenType::InlineMacroClose,
                "]".to_string(),
                Some("]".to_string()),
                1,
                25,
                25,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[rstest]
    #[case("http://example.com")]
    #[case("http://example.com/")]
    #[case("http://example.com/?next=foo")]
    #[case("http://example.com:80/")]
    #[case("http://example.com/foo.html#bar")]
    fn link_no_attribute_fencing(#[case] link: &str) {
        let markup = &format!("Somx {link}");
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Somx ".to_string(),
                Some("Somx ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(
                TokenType::Hyperlink,
                link.to_string(),
                Some(link.to_string()),
                1,
                6,
                5 + link.len(),
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    /// A bare URL at the end of a line stops at the newline instead of running into the lines
    /// that follow it
    #[rstest]
    #[case::plain("http://example.com", "http://example.com")]
    #[case::trailing_slash("http://example.com/bar/", "http://example.com/bar/")]
    #[case::sentence_end("http://example.com.", "http://example.com")]
    #[case::allowed_punctuation("http://example.com/?a=b", "http://example.com/?a=b")]
    fn link_no_attribute_fencing_end_of_line(#[case] line: &str, #[case] link: &str) {
        let markup = format!("Somx {line}\nnext\n\n== Title\n");
        let tokens: Vec<Token> = Scanner::new(&markup).filter_map(Result::ok).collect();
        let hyperlink_idx = tokens
            .iter()
            .position(|t| t.token_type() == TokenType::Hyperlink)
            .expect("no hyperlink token");
        assert_eq!(tokens[hyperlink_idx].lexeme, link);
        assert!(
            tokens
                .iter()
                .any(|t| t.token_type() == TokenType::Heading2 && t.line == 4),
            "{tokens:?}"
        );
    }

    /// The same, with Windows (CRLF) line endings: the `\r` is not part of the link
    #[rstest]
    #[case::plain("http://example.com", "http://example.com")]
    #[case::sentence_end("http://example.com.", "http://example.com")]
    #[case::allowed_punctuation("http://example.com/?a=b", "http://example.com/?a=b")]
    fn link_no_attribute_fencing_end_of_crlf_line(#[case] line: &str, #[case] link: &str) {
        let markup = format!("Somx {line}\r\nnext\r\n");
        let tokens: Vec<Token> = Scanner::new(&markup).filter_map(Result::ok).collect();
        let hyperlink = tokens
            .iter()
            .find(|t| t.token_type() == TokenType::Hyperlink)
            .expect("no hyperlink token");
        assert_eq!(hyperlink.lexeme, link);
    }

    #[test]
    fn link_no_attribute_fencing_more_text() {
        let markup = "Somx http://example.com Somx";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Somx ".to_string(),
                Some("Somx ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(
                TokenType::Hyperlink,
                "http://example.com".to_string(),
                Some("http://example.com".to_string()),
                1,
                6,
                23,
            ),
            Token::new_default(
                TokenType::Text,
                " Somx".to_string(),
                Some(" Somx".to_string()),
                1,
                24,
                28,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[rstest]
    #[case(". ")]
    #[case(".")]
    #[case(", ")]
    #[case(",")]
    #[case(": ")]
    #[case("? ")]
    #[case(". ")]
    #[case(":")]
    #[case("?")]
    #[case(".")]
    fn link_no_attribute_fencing_sentence(#[case] end_: &str) {
        let markup = format!("Somx http://example.com{}", end_);
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Somx ".to_string(),
                Some("Somx ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(
                TokenType::Hyperlink,
                "http://example.com".to_string(),
                Some("http://example.com".to_string()),
                1,
                6,
                23,
            ),
            Token::new_default(
                TokenType::Text,
                end_.to_string(),
                Some(end_.to_string()),
                1,
                24,
                23 + end_.len(),
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn block_image_with_alt() {
        let markup = "image::path/to/img.png[alt text]";
        let expected_tokens = vec![Token::new_default(
            TokenType::BlockImageMacro,
            "image::path/to/img.png[alt text]".to_string(),
            Some("image::path/to/img.png[alt text]".to_string()),
            1,
            1,
            32,
        )];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn inline_image() {
        let markup = "Somx image:path/to/img.png[]";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Somx ".to_string(),
                Some("Somx ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(
                TokenType::InlineImageMacro,
                "image:path/to/img.png[]".to_string(),
                Some("image:path/to/img.png[]".to_string()),
                1,
                6,
                28,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn no_inline_image_if_double_colon() {
        let markup = "Somx image::bar.png[]";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Somx ".to_string(),
                Some("Somx ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(
                TokenType::Text,
                "imag".to_string(),
                Some("imag".to_string()),
                1,
                6,
                9,
            ),
            Token::new_default(
                TokenType::Text,
                "e".to_string(),
                Some("e".to_string()),
                1,
                10,
                10,
            ),
            Token::new_default(
                TokenType::Text,
                ":".to_string(),
                Some(":".to_string()),
                1,
                11,
                11,
            ),
            Token::new_default(
                TokenType::Text,
                ":bar.".to_string(),
                Some(":bar.".to_string()),
                1,
                12,
                16,
            ),
            Token::new_default(
                TokenType::Text,
                "png".to_string(),
                Some("png".to_string()),
                1,
                17,
                19,
            ),
            Token::new_default(
                TokenType::Text,
                "[".to_string(),
                Some("[".to_string()),
                1,
                20,
                20,
            ),
            Token::new_default(
                TokenType::InlineMacroClose,
                "]".to_string(),
                Some("]".to_string()),
                1,
                21,
                21,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn inline_style_in_line() {
        let markup = "Somx [.style]#foo#";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Somx ".to_string(),
                Some("Somx ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(
                TokenType::InlineStyle,
                "[.style]".to_string(),
                Some("[.style]".to_string()),
                1,
                6,
                13,
            ),
            Token::new_default(TokenType::Mark, "#".to_string(), None, 1, 14, 14),
            Token::new_default(
                TokenType::Text,
                "foo".to_string(),
                Some("foo".to_string()),
                1,
                15,
                17,
            ),
            Token::new_default(TokenType::Mark, "#".to_string(), None, 1, 18, 18),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn inline_style_new_line() {
        let markup = "\n[.style]#foox#";
        let expected_tokens = vec![
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new_default(
                TokenType::InlineStyle,
                "[.style]".to_string(),
                Some("[.style]".to_string()),
                2,
                1,
                8,
            ),
            Token::new_default(TokenType::Mark, "#".to_string(), None, 2, 9, 9),
            Token::new_default(
                TokenType::Text,
                "foox".to_string(),
                Some("foox".to_string()),
                2,
                10,
                13,
            ),
            Token::new_default(TokenType::Mark, "#".to_string(), None, 2, 14, 14),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }
    #[test]
    fn inline_style_new_block() {
        let markup = "\n\n[.style]#foo#";
        let expected_tokens = vec![
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 2, 1, 1),
            Token::new_default(
                TokenType::InlineStyle,
                "[.style]".to_string(),
                Some("[.style]".to_string()),
                3,
                1,
                8,
            ),
            Token::new_default(TokenType::Mark, "#".to_string(), None, 3, 9, 9),
            Token::new_default(
                TokenType::Text,
                "foo".to_string(),
                Some("foo".to_string()),
                3,
                10,
                12,
            ),
            Token::new_default(TokenType::Mark, "#".to_string(), None, 3, 13, 13),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn inline_charref() {
        let markup = "bar&mdash;bar";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                1,
                3,
            ),
            Token::new_default(
                TokenType::CharRef,
                "&mdash;".to_string(),
                Some("&mdash;".to_string()),
                1,
                4,
                10,
            ),
            Token::new_default(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                11,
                13,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn inline_charref_spaces() {
        let markup = "bar &mdash; bar";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "bar ".to_string(),
                Some("bar ".to_string()),
                1,
                1,
                4,
            ),
            Token::new_default(
                TokenType::CharRef,
                "&mdash;".to_string(),
                Some("&mdash;".to_string()),
                1,
                5,
                11,
            ),
            Token::new_default(
                TokenType::Text,
                " bar".to_string(),
                Some(" bar".to_string()),
                1,
                12,
                15,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn document_attribute_name_value() {
        let markup = ":foo: bar\n";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Attribute,
                String::from(":foo: bar"),
                None,
                1,
                1,
                9,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 10, 10),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn inline_attr_reference() {
        let markup = "bar{foo}bar";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                1,
                3,
            ),
            Token::new_default(
                TokenType::AttributeReference,
                "{foo}".to_string(),
                None,
                1,
                4,
                8,
            ),
            Token::new_default(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                9,
                11,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn cross_reference() {
        let markup = "<<foo_bar>>";
        let expected_tokens = vec![Token::new_default(
            TokenType::CrossReference,
            "<<foo_bar>>".to_string(),
            Some("<<foo_bar>>".to_string()),
            1,
            1,
            11,
        )];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn simple_include() {
        let markup = "include::partial.adoc[]";
        let expected_tokens = vec![Token::new_default(
            TokenType::Include,
            "include::partial.adoc[]".to_string(),
            Some("include::partial.adoc[]".to_string()),
            1,
            1,
            23,
        )];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[rstest]
    #[case::newline("\n")]
    #[case::start_of_file("")]
    fn block_anchor(#[case] beginning: &str) {
        let mut addition: usize = 0;
        let mut expected_tokens: Vec<Token> = vec![];
        if beginning == "\n" {
            addition = 1;
            expected_tokens = vec![Token::new_default(
                TokenType::NewLineChar,
                "\n".to_string(),
                None,
                1,
                1,
                1,
            )];
        }
        let markup = format!("{beginning}[[foo]]\n");
        expected_tokens.extend(vec![
            Token::new_default(
                TokenType::BlockAnchor,
                "[[foo]]".to_string(),
                Some("[[foo]]".to_string()),
                1 + addition,
                1,
                7,
            ),
            Token::new_default(
                TokenType::NewLineChar,
                "\n".to_string(),
                None,
                1 + addition,
                8,
                8,
            ),
        ]);
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn simple_table() {
        let markup = "[cols=\"1,1\"]\n|===\n|cell one\n|cell two\n|===";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::ElementAttributes,
                "[cols=\"1,1\"]".to_string(),
                Some("[cols=\"1,1\"]".to_string()),
                1,
                1,
                12,
            ),
            newline_token_at(1, 13),
            Token::new_default(TokenType::Table, "|===".to_string(), None, 2, 1, 4),
            newline_token_at(2, 5),
            Token::new_default(
                TokenType::TableCell,
                "|cell one".to_string(),
                Some("|cell one".to_string()),
                3,
                1,
                9,
            ),
            newline_token_at(3, 10),
            Token::new_default(
                TokenType::TableCell,
                "|cell two".to_string(),
                Some("|cell two".to_string()),
                4,
                1,
                9,
            ),
            newline_token_at(4, 10),
            Token::new_default(TokenType::Table, "|===".to_string(), None, 5, 1, 4),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn simple_table_cols_same_line() {
        let markup = "[cols=\"1,1\"]\n|===\n|cell one |cell two\n|===";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::ElementAttributes,
                "[cols=\"1,1\"]".to_string(),
                Some("[cols=\"1,1\"]".to_string()),
                1,
                1,
                12,
            ),
            newline_token_at(1, 13),
            Token::new_default(TokenType::Table, "|===".to_string(), None, 2, 1, 4),
            newline_token_at(2, 5),
            Token::new_default(
                TokenType::TableCell,
                "|cell one ".to_string(),
                Some("|cell one ".to_string()),
                3,
                1,
                10,
            ),
            Token::new_default(
                TokenType::TableCell,
                "|cell two".to_string(),
                Some("|cell two".to_string()),
                3,
                11,
                19,
            ),
            newline_token_at(3, 20),
            Token::new_default(TokenType::Table, "|===".to_string(), None, 4, 1, 4),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    // this previously panicked at byte index 13 because it is not a char boundary; should now pass
    #[test]
    fn scan_odd_boundaried_text() {
        let markup = "rxgular shis…\n";
        let expected_tokens = vec![
            Token {
                token_type: TokenType::Text,
                lexeme: "rxgular s".into(),
                literal: Some("rxgular s".into()),
                line: 1,
                startcol: 1,
                endcol: 9,
                file_stack: vec![],
            },
            Token {
                token_type: TokenType::Text,
                lexeme: "h".into(),
                literal: Some("h".into()),
                line: 1,
                startcol: 10,
                endcol: 10,
                file_stack: vec![],
            },
            Token {
                token_type: TokenType::Text,
                lexeme: "is…".into(),
                literal: Some("is…".into()),
                line: 1,
                startcol: 11,
                endcol: 15,
                file_stack: vec![],
            },
            Token {
                token_type: TokenType::NewLineChar,
                lexeme: "\n".into(),
                literal: None,
                line: 1,
                startcol: 16,
                endcol: 16,
                file_stack: vec![],
            },
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn picks_up_code_callouts_behind_inline_comment() {
        let markup = "bar // <1>\n";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "bar // ".to_string(),
                Some("bar // ".to_string()),
                1,
                1,
                7,
            ),
            Token::new_default(
                TokenType::CodeCallout,
                "<1>".to_string(),
                Some("<1>".to_string()),
                1,
                8,
                10,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 11, 11),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn code_callout_list() {
        let markup = "<1> Bar";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::CodeCalloutListItem,
                "<1> ".to_string(),
                Some("<1> ".to_string()),
                1,
                1,
                4,
            ),
            Token::new_default(
                TokenType::Text,
                "Bar".to_string(),
                Some("Bar".to_string()),
                1,
                5,
                7,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn typographers_apostrophe() {
        let markup = "Sam's ";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Sam".to_string(),
                Some("Sam".to_string()),
                1,
                1,
                3,
            ),
            Token::new_default(
                TokenType::CharRef,
                "'".to_string(),
                Some("&#8217;".to_string()),
                1,
                4,
                4,
            ),
            Token::new_default(
                TokenType::Text,
                "s ".to_string(),
                Some("s ".to_string()),
                1,
                5,
                6,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn typographers_apostrophe_newline() {
        let markup = "Sam's\n";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Sam".to_string(),
                Some("Sam".to_string()),
                1,
                1,
                3,
            ),
            Token::new_default(
                TokenType::CharRef,
                "'".to_string(),
                Some("&#8217;".to_string()),
                1,
                4,
                4,
            ),
            Token::new_default(
                TokenType::Text,
                "s".to_string(),
                Some("s".to_string()),
                1,
                5,
                5,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 6, 6),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[test]
    fn typographers_apostrophe_eof() {
        let markup = "Sam's";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Sam".to_string(),
                Some("Sam".to_string()),
                1,
                1,
                3,
            ),
            Token::new_default(
                TokenType::CharRef,
                "'".to_string(),
                Some("&#8217;".to_string()),
                1,
                4,
                4,
            ),
            Token::new_default(
                TokenType::Text,
                "s".to_string(),
                Some("s".to_string()),
                1,
                5,
                5,
            ),
        ];
        scan_and_assert_eq(markup, expected_tokens);
    }

    #[rstest]
    #[case::double_quotes("\"`", "`\"", TokenType::OpenDoubleQuote, TokenType::CloseDoubleQuote)]
    #[case::single_quotes("'`", "`'", TokenType::OpenSingleQuote, TokenType::CloseSingleQuote)]
    fn typographers_quotes(
        #[case] open_markup: &str,
        #[case] close_markup: &str,
        #[case] open_token: TokenType,
        #[case] close_token: TokenType,
    ) {
        let markup = format!("{}bar{}", open_markup, close_markup);
        let expected_tokens = vec![
            Token::new_default(
                open_token,
                open_markup.to_string(),
                Some(open_markup.to_string()),
                1,
                1,
                2,
            ),
            Token::new_default(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                3,
                5,
            ),
            Token::new_default(
                close_token,
                close_markup.to_string(),
                Some(close_markup.to_string()),
                1,
                6,
                7,
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case("(C)", "&#169;")]
    #[case("(R)", "&#174;")]
    #[case("(TM)", "&#8482;")]
    #[case("...", "&#8230;")]
    #[case("->", "&#8594;")]
    #[case("=>", "&#8658;")]
    #[case("<-", "&#8592;")]
    #[case("<=", "&#8656;")]
    // note that ^ and ~ transformations are handled better by the parser
    fn character_replacements_minus_emdash(#[case] markup: &str, #[case] replacement: &str) {
        let expected_tokens = vec![Token::new_default(
            TokenType::CharRef,
            markup.to_string(),
            Some(replacement.to_string()),
            1,
            1,
            markup.len(),
        )];
        scan_and_assert_eq(markup, expected_tokens);
    }
    #[test]
    // note that ^ and ~ transformations are handled better by the parser
    fn character_replacements_emdash_alone() {
        let markup = "--".to_string();
        let expected_tokens = vec![Token::new_default(
            TokenType::CharRef,
            "--".to_string(),
            Some("&#8212;".to_string()),
            1,
            1,
            markup.len(),
        )];
        scan_and_assert_eq(&markup, expected_tokens);
    }
    #[test]
    fn character_replacements_emdash_space() {
        let markup = "-- ".to_string();
        let expected_tokens = vec![
            Token::new_default(
                TokenType::CharRef,
                "--".to_string(),
                Some("&#8212;".to_string()),
                1,
                1,
                2,
            ),
            Token::new_default(
                TokenType::Text,
                " ".to_string(),
                Some(" ".to_string()),
                1,
                3,
                3,
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case("1", true)]
    #[case("a", true)]
    #[case("(", false)]
    fn character_replacements_emdash_alpha_numeric(#[case] next_char: &str, #[case] pass: bool) {
        let markup = format!("--{}", next_char);
        let expected = Token::new_default(
            TokenType::CharRef,
            "--".to_string(),
            Some("&#8212;".to_string()),
            1,
            1,
            2,
        );

        let scanned: Vec<Token> = Scanner::new(&markup)
            .filter_map(|result| result.as_ref().ok().cloned())
            .collect();
        if pass {
            assert_eq!(scanned.first().unwrap(), &expected)
        } else {
            assert_ne!(scanned.first().unwrap(), &expected)
        }
    }

    #[test]
    fn character_replacements_emdash_inline() {
        let markup = "This --".to_string();
        let expected = Token::new_default(
            TokenType::CharRef,
            "--".to_string(),
            Some("&#8212;".to_string()),
            1,
            6,
            7,
        );
        let mut scanned: Vec<Token> = Scanner::new(&markup)
            .filter_map(|result| result.as_ref().ok().cloned())
            .collect();
        scanned.pop(); // get rid of EOF token
        assert_eq!(scanned.last().unwrap(), &expected)
    }

    #[rstest]
    #[case("tag", TokenType::StartTag)]
    #[case("end", TokenType::EndTag)]
    fn simple_tag(#[case] marker: &str, #[case] tag: TokenType) {
        let markup = format!("{}::foo[]", marker);
        let expected = Token::new_default(tag, markup.clone(), Some(markup.clone()), 1, 1, 10);
        let mut scanned: Vec<Token> = Scanner::new(&markup)
            .filter_map(|result| result.as_ref().ok().cloned())
            .collect();
        scanned.pop(); // get rid of EOF token
        assert_eq!(scanned.last().unwrap(), &expected)
    }

    #[rstest]
    #[case("<!-- ", " -->")] // xml style comments
    #[case("// ", "")] // c-style comments
    #[case("# ", "")] // python-style comments
    fn contextual_tag_start(#[case] context_start: &str, #[case] context_end: &str) {
        let tag = String::from("tag::foo[]");
        let markup = format!("{}{}{}", context_start, tag, context_end);
        let tag_col_start = context_start.len() + 1;
        let tag_col_end = tag_col_start + tag.len() - 1;
        let expected = Token::new_default(
            TokenType::StartTag,
            tag.clone(),
            Some(tag.clone()),
            1,
            tag_col_start,
            tag_col_end,
        );
        let scanned: Vec<Token> = Scanner::new(&markup)
            .filter_map(|result| result.as_ref().ok().cloned())
            .collect();
        assert!(scanned.contains(&expected))
    }

    #[rstest]
    #[case("<!-- ", " -->")] // xml style comments
    #[case("// ", "")] // c-style comments
    #[case("# ", "")] // python-style comments
    fn contextual_tag_end(#[case] context_start: &str, #[case] context_end: &str) {
        let tag = String::from("end::foo[]");
        let markup = format!("\n{}{}{}", context_start, tag, context_end);
        let tag_col_start = context_start.len() + 1;
        let tag_col_end = tag_col_start + tag.len() - 1;
        let expected = vec![
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new_default(
                TokenType::EndTag,
                tag.clone(),
                Some(tag.clone()),
                2,
                tag_col_start,
                tag_col_end,
            ),
        ];
        let mut scanner = Scanner::new(&markup);
        scanner.expecting_tag_end = true;
        let mut scanned: Vec<Token> = scanner
            .filter_map(|result| result.as_ref().ok().cloned())
            .collect();
        scanned.pop(); // remove EOF
        assert_eq!(scanned, expected)
    }
}
