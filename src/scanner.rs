use core::str;

use log::error;

use crate::tokens::{Token, TokenType};

#[derive(Debug)]
pub struct Scanner<'a> {
    pub source: &'a str,
    start: usize,
    startcol: usize,
    current: usize,
    line: usize,
    file_stack: Vec<String>,
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.is_at_end() {
            self.start = self.current;
            return Some(self.scan_token());
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
            file_stack,
        }
    }

    fn scan_token(&mut self) -> Token {
        let c = self.source.as_bytes()[self.current] as char;
        self.current += 1; // this instead of the "advance" function in "Crafting Interpreters"

        match c {
            '\n' => self.add_token(TokenType::NewLineChar, false, 1),

            '\'' => {
                if self.starts_repeated_char_line(c, 3) {
                    self.current += 2;
                    self.add_token(TokenType::ThematicBreak, false, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            '<' => {
                if self.starts_repeated_char_line(c, 3) {
                    self.current += 2;
                    self.add_token(TokenType::PageBreak, false, 0)
                } else if self.peek() == '<' {
                    self.add_cross_reference()
                } else if self.starts_code_callout() {
                    self.current += 1; // add the trailing '>' char
                    self.add_token(TokenType::CodeCallout, true, 0)
                // else if ...
                } else {
                    self.add_text_until_next_markup()
                }
            }
            // potential block delimiter chars get treated similarly
            '+' | '*' | '-' | '_' | '/' | '=' | '.' => {
                if self.starts_new_line() && self.starts_repeated_char_line(c, 4) {
                    self.current += 3; // the remaining repeated chars
                    self.add_token(TokenType::block_from_char(c), false, 0)
                } else {
                    match c {
                        '=' => {
                            // possible heading
                            if self.starts_new_line() {
                                self.add_heading()
                            } else {
                                self.add_text_until_next_markup()
                            }
                        }
                        '-' => {
                            // check if it's an open block
                            if self.starts_new_line() && self.starts_repeated_char_line(c, 2) {
                                self.current += 1;
                                // since we consume the newline as a part of the block, add a line
                                self.add_token(TokenType::OpenBlock, false, 0)
                            } else {
                                self.add_text_until_next_markup()
                            }
                        }
                        '*' => {
                            // check if it's a list item
                            if self.starts_new_line() && self.peek() == ' ' {
                                self.add_list_item(TokenType::UnorderedListItem)
                            } else {
                                self.handle_inline_formatting(
                                    c,
                                    TokenType::Strong,
                                    TokenType::UnconstrainedStrong,
                                )
                            }
                        }
                        '/' => {
                            if self.starts_new_line() && self.peek() == '/' {
                                while self.peek() != '\n' && !self.is_at_end() {
                                    self.current += 1;
                                }
                                self.add_token(TokenType::Comment, true, 0)
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
                                self.add_text_until_next_markup()
                            }
                        }
                        '_' => self.handle_inline_formatting(
                            c,
                            TokenType::Emphasis,
                            TokenType::UnconstrainedEmphasis,
                        ),
                        // ordered list item or section title
                        '.' => {
                            if self.starts_new_line() {
                                if self.peek() == ' ' {
                                    self.add_list_item(TokenType::OrderedListItem)
                                } else {
                                    self.add_token(TokenType::BlockLabel, false, 0)
                                }
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

            '`' => self.handle_inline_formatting(
                c,
                TokenType::Monospace,
                TokenType::UnconstrainedMonospace,
            ),
            '^' => self.add_token(TokenType::Superscript, false, 0),
            '~' => self.add_token(TokenType::Subscript, false, 0),
            '#' => self.handle_inline_formatting(c, TokenType::Mark, TokenType::UnconstrainedMark),
            ':' => {
                if self.starts_new_line() && self.starts_attr() {
                    while self.peek() != '\n' {
                        // TK line continuation
                        self.current += 1
                    }
                    self.add_token(TokenType::Attribute, false, 0)
                } else if self.peek_back() != ' ' && [": ", ":\n"].contains(&self.peeks_ahead(2)) {
                    self.current += 2;
                    self.add_token(TokenType::DescriptionListMarker, false, 0)
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
            _ => self.add_text_until_next_markup(),
        }
    }

    fn add_token(
        &mut self,
        token_type: TokenType,
        include_literal: bool,
        advance_line_after: usize,
    ) -> Token {
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
        token
    }

    fn add_heading(&mut self) -> Token {
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
                error!("Invalid headling level: line {}", self.line);
                std::process::exit(1)
            }
        }
    }

    /// adds the list item token, then includes the rest of the list item (until a new block or
    /// another list item marker) in an Text
    fn add_list_item(&mut self, list_item_token: TokenType) -> Token {
        self.current += 1; // advance past the space, which we'll include in the token lexeme
        self.add_token(list_item_token, false, 0)
    }

    /// Adds the block image, consuming the target as well as any attributes
    fn add_block_image(&mut self) -> Token {
        while self.peek() != ']' {
            self.current += 1
        }
        self.current += 1; // consume the ']' char
        self.add_token(TokenType::BlockImageMacro, true, 0)
    }

    /// Adds the block image, consuming the target as well as any attributes
    fn add_inline_image(&mut self) -> Token {
        while self.peek() != ']' {
            self.current += 1
        }
        self.current += 1; // consume the ']' char
        self.add_token(TokenType::InlineImageMacro, true, 0)
    }

    /// Adds the include image, consuming the target as well as any attributes
    fn add_include(&mut self) -> Token {
        while self.peek() != ']' {
            self.current += 1
        }
        self.current += 1; // consume the ']' char
        self.add_token(TokenType::Include, true, 0)
    }

    /// Adds contents into a TableCell token, regardless of other formatting (simpler to re-scan later)
    fn add_table_cell(&mut self) -> Token {
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

    /// Adds the link, consuming the target but not any attributes
    fn add_link(&mut self) -> Token {
        while self.peek() != '[' {
            self.current += 1
        }
        self.current += 1; // consume the '[' char
        self.add_token(TokenType::LinkMacro, true, 0)
    }

    fn add_inline_style(&mut self) -> Token {
        while self.peek() != ']' {
            self.current += 1
        }
        self.current += 1; // consume the ']' char
        self.add_token(TokenType::InlineStyle, true, 0)
    }

    // adds block anchors, e.g., "\n[[some_block_id]]\n"
    fn add_cross_reference(&mut self) -> Token {
        while self.peeks_ahead(2) != ">>" && !self.is_at_end() {
            self.current += 1
        }
        self.current += 2; // consume the '>>' chars
        self.add_token(TokenType::CrossReference, true, 0)
    }

    // adds block anchors, e.g., "\n[[some_block_id]]\n"
    fn add_block_anchor(&mut self) -> Token {
        while self.peeks_ahead(3) != "]]\n" && !self.is_at_end() {
            self.current += 1
        }
        self.current += 2; // consume the ']]' chars, NOT newline
        self.add_token(TokenType::BlockAnchor, true, 0)
    }

    fn add_text_until_next_markup(&mut self) -> Token {
        // "inline" chars that could be markup; the newline condition prevents
        // capturing significant block markup chars
        // Chars: newline, bold, italic, code, super, subscript, footnote, pass, link, end inline macro, definition list marker, highlighted, inline admonition initial chars, inline images
        while ![
            '\n', '*', '_', '`', '^', '~', 'f', 'p', 'h', ']', '[', ':', '#', 'N', 'T', 'I', 'C',
            'W', '&', '{', '+', 'i', '<',
        ]
        .contains(&self.peek())
            && !self.is_at_end()
        {
            self.current += 1;
        }
        self.add_token(TokenType::Text, true, 0)
    }

    fn starts_new_block(&self) -> bool {
        self.start == 0
            || self.start >= 2 // guarding against very short, silly documents (tests)
                && self.source.as_bytes()[self.start - 2] == b'\n'
                && self.source.as_bytes()[self.start - 1] == b'\n'
    }

    fn starts_new_line(&self) -> bool {
        self.start == 0 || self.source.as_bytes()[self.start - 1] == b'\n'
    }

    fn starts_repeated_char_line(&self, c: char, delimiter_len: usize) -> bool {
        let mut expected_block = String::from(c).repeat(delimiter_len);
        expected_block.push('\n');

        self.current + delimiter_len <= self.source.len()
            && self.source[self.start..self.current + delimiter_len] == expected_block
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
    ) -> Token {
        // punctuation
        if [
            ' ', '\0', '.', ',', ';', ':', '\n', ')', '"', '!', '?', '\'', ']', '…', '“', '”', '‘',
            '’',
        ]
        .contains(&self.peek())
            || [' ', '\n', '\0', ']', '(', '"', '['].contains(&self.peek_back()) && self.peek() != c
        {
            self.add_token(constrained, false, 0)
        } else if self.peek() == c {
            // we've got an unconstrained version
            self.current += 1;
            self.add_token(unconstrained, false, 0)
        } else {
            self.add_text_until_next_markup()
        }
    }

    /// Checks for lines such as [quote], [verse, Mary Oliver], [source, python], etc.
    fn starts_attribution_line(&mut self) -> bool {
        let current_placeholder = self.current;
        while self.peek() != '\n' && !self.is_at_end() {
            self.current += 1;
        }
        if self.starts_new_block() && self.source.as_bytes()[self.current - 1] as char == ']' {
            // i.e., the end of an attribute list line
            true
        } else {
            self.current = current_placeholder;
            false
        }
    }
    fn starts_code_callout(&mut self) -> bool {
        while self.peek() != '>' {
            if self.peek().is_digit(10) {
                self.current += 1;
            } else {
                return false;
            }
        }
        return true;
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
}

#[cfg(test)]
mod tests {
    use std::char;

    use crate::tokens::TokenType;
    use rstest::rstest;

    use super::*;

    fn scan_and_assert_eq(markup: &str, expected_tokens: Vec<Token>) {
        let s = Scanner::new(markup);
        assert_eq!(s.collect::<Vec<Token>>(), expected_tokens);
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
    #[case("++++\n".to_string(), TokenType::PassthroughBlock)]
    #[case("****\n".to_string(), TokenType::SidebarBlock)]
    #[case("----\n".to_string(), TokenType::SourceBlock)]
    #[case("____\n".to_string(), TokenType::QuoteVerseBlock)]
    #[case("____\n".to_string(), TokenType::QuoteVerseBlock)]
    #[case("====\n".to_string(), TokenType::ExampleBlock)]
    #[case("....\n".to_string(), TokenType::LiteralBlock)]
    fn fenced_block_delimiter_start(#[case] markup: String, #[case] expected_token: TokenType) {
        let expected_tokens = vec![
            Token::new_default(
                expected_token,
                markup.clone()[..4].to_string(),
                None,
                1,
                1,
                4,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 5, 5),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case("\n\n++++\n".to_string(), TokenType::PassthroughBlock)]
    #[case("\n\n****\n".to_string(), TokenType::SidebarBlock)]
    #[case("\n\n----\n".to_string(), TokenType::SourceBlock)]
    #[case("\n\n____\n".to_string(), TokenType::QuoteVerseBlock)]
    #[case("\n\n////\n".to_string(), TokenType::CommentBlock)]
    #[case("\n\n====\n".to_string(), TokenType::ExampleBlock)]
    #[case("\n\n....\n".to_string(), TokenType::LiteralBlock)]
    fn fenced_block_delimiter_new_block(#[case] markup: String, #[case] expected_token: TokenType) {
        let expected_tokens = vec![
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 2, 1, 1),
            Token::new_default(
                expected_token,
                markup.clone()[2..6].to_string(),
                None,
                3,
                1,
                4,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 3, 5, 5),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn open_block_beginning() {
        let markup = "--\n".to_string();
        let expected_tokens = vec![
            Token::new_default(
                TokenType::OpenBlock,
                markup.clone()[..2].to_string(),
                None,
                1,
                1,
                2,
            ),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 3, 3),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn open_block_new_block() {
        let markup = "\n\n--\n".to_string();
        let expected_tokens = vec![
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 2, 1, 1),
            Token::new_default(TokenType::OpenBlock, "--".to_string(), None, 3, 1, 2),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 3, 3, 3),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
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

    #[test]
    fn block_introduction() {
        // skip the "i"
        let markup = ".Some ttle\n".to_string();
        let title = "Some ttle".to_string();
        let expected_tokens = vec![
            Token::new_default(TokenType::BlockLabel, ".".to_string(), None, 1, 1, 1),
            Token::new_default(TokenType::Text, title.clone(), Some(title), 1, 2, 10),
            Token::new_default(TokenType::NewLineChar, "\n".to_string(), None, 1, 11, 11),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
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
        scan_and_assert_eq(&markup, expected_tokens);
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
    #[case('^', TokenType::Superscript)]
    #[case('~', TokenType::Subscript)]
    #[case('#', TokenType::Mark)]
    fn inline_formatting(#[case] markup_char: char, #[case] expected_token: TokenType) {
        let markup = format!("Some {}bar{} bar.", markup_char, markup_char);
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
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
    fn inline_formatting_doubles(#[case] markup_str: String, #[case] expected_token: TokenType) {
        let markup = format!("Some{}bar{}bar.", markup_str, markup_str);
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Some".to_string(),
                Some("Some".to_string()),
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
    #[case("footnote:[", TokenType::FootnoteMacro)]
    #[case("pass:[", TokenType::PassthroughInlineMacro)]
    fn inline_macros(#[case] markup_check: &str, #[case] expected_token: TokenType) {
        let markup = format!("Some {}bar]", &markup_check);
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
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
        let markup = "Term:: DS";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Term".to_string(),
                Some("Term".to_string()),
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
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn description_list_mark_newline() {
        // make it "DS" instead of "Description" to avoid the "i" delimiting the text
        let markup = "Term::\nDS";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Term".to_string(),
                Some("Term".to_string()),
                1,
                1,
                4,
            ),
            Token::new_default(
                TokenType::DescriptionListMarker,
                "::\n".to_string(),
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
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn description_list_mark_not_if_space_before() {
        let markup = "Term :: bar";

        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Term ".to_string(),
                Some("Term ".to_string()),
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
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn link_with_text() {
        let markup = "Some http://example.com[bar]";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
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
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn link_no_text() {
        let markup = "Some http://example.com[]";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
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
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn inline_image() {
        let markup = "Some image:path/to/img.png[]";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
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
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn no_inline_image_if_double_colon() {
        let markup = "Some image::bar.png[]";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
                1,
                1,
                5,
            ),
            Token::new_default(
                TokenType::Text,
                "image".to_string(),
                Some("image".to_string()),
                1,
                6,
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
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn inline_style_in_line() {
        let markup = "Some [.style]#text#";
        let expected_tokens = vec![
            Token::new_default(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
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
                "text".to_string(),
                Some("text".to_string()),
                1,
                15,
                18,
            ),
            Token::new_default(TokenType::Mark, "#".to_string(), None, 1, 19, 19),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn inline_style_new_line() {
        let markup = "\n[.style]#text#";
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
                "text".to_string(),
                Some("text".to_string()),
                2,
                10,
                13,
            ),
            Token::new_default(TokenType::Mark, "#".to_string(), None, 2, 14, 14),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }
    #[test]
    fn inline_style_new_block() {
        let markup = "\n\n[.style]#text#";
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
                "text".to_string(),
                Some("text".to_string()),
                3,
                10,
                13,
            ),
            Token::new_default(TokenType::Mark, "#".to_string(), None, 3, 14, 14),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
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
        scan_and_assert_eq(&markup, expected_tokens);
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
        scan_and_assert_eq(&markup, expected_tokens);
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
        scan_and_assert_eq(&markup, expected_tokens);
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
        scan_and_assert_eq(&markup, expected_tokens);
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
        scan_and_assert_eq(&markup, expected_tokens);
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
        scan_and_assert_eq(&markup, expected_tokens);
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
        scan_and_assert_eq(&markup, expected_tokens);
    }

    // this previously panicked at byte index 13 because it is not a char boundary; should now pass
    #[test]
    fn scan_odd_boundaried_text() {
        let markup = "regular shit…\n";
        let expected_tokens = vec![
            Token {
                token_type: TokenType::Text,
                lexeme: "regular s".into(),
                literal: Some("regular s".into()),
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
                lexeme: "it…".into(),
                literal: Some("it…".into()),
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
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn picks_up_code_callouts_behind_inline_comment() {
        let markup = "bar // <1>";
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
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }
}
