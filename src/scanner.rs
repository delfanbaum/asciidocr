use core::{panic, str};

use crate::tokens::{Token, TokenType};

#[derive(Debug)]
pub struct Scanner<'a> {
    pub source: &'a str,
    start: usize,
    startcol: usize,
    current: usize,
    line: usize,
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
                } else {
                    self.add_text_until_next_markup()
                }
            }
            // potential block delimiter chars get treated similarly
            '+' | '*' | '-' | '_' | '/' | '=' => {
                if self.starts_new_block() && self.starts_repeated_char_line(c, 4) {
                    self.current += 3; // the remaining repeated chars
                    self.add_token(TokenType::block_from_char(c), false, 0)
                } else {
                    match c {
                        '=' => {
                            // possible heading
                            if self.starts_new_block() {
                                self.add_heading()
                            } else {
                                self.add_text_until_next_markup()
                            }
                        }
                        '-' => {
                            // check if it's an open block
                            if self.starts_new_block() && self.starts_repeated_char_line(c, 2) {
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
                                self.add_token(TokenType::Bold, false, 0)
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
                            } else {
                                self.add_text_until_next_markup()
                            }
                        }
                        '_' => self.add_token(TokenType::Italic, false, 0),
                        _ => self.add_text_until_next_markup(),
                    }
                }
            }

            // ordered list item or section title
            '.' => {
                if self.starts_new_block() {
                    if self.peek() == ' ' {
                        self.add_list_item(TokenType::OrderedListItem)
                    } else {
                        self.add_token(TokenType::BlockLabel, false, 0)
                    }
                } else {
                    self.add_text_until_next_markup()
                }
            }

            '[' => {
                // role, quote, verse, source, etc
                if self.starts_attribute_line() {
                    match self.source.as_bytes()[self.start + 1] as char {
                        'q' => self.add_token(TokenType::Blockquote, true, 0),
                        'v' => self.add_token(TokenType::Verse, true, 0),
                        's' => self.add_token(TokenType::Source, true, 0),
                        'N' => self.add_token(TokenType::Note, true, 0),
                        'T' => self.add_token(TokenType::Tip, true, 0),
                        'I' => self.add_token(TokenType::Important, true, 0),
                        'C' => self.add_token(TokenType::Caution, true, 0),
                        'W' => self.add_token(TokenType::Warning, true, 0),
                        _ => self.add_text_until_next_markup(),
                    }
                } else if self.peek() == '.' {
                    self.add_inline_style()
                } else {
                    self.add_text_until_next_markup()
                }
            }

            '`' => self.add_token(TokenType::Monospace, false, 0),
            '^' => self.add_token(TokenType::Superscript, false, 0),
            '~' => self.add_token(TokenType::Subscript, false, 0),
            '#' => self.add_token(TokenType::Highlighted, false, 0),
            ':' => {
                if self.peek_back() != ' ' && self.peeks_ahead(2) == ": " {
                    self.current += 2;
                    self.add_token(TokenType::DefListMark, false, 0)
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
            'N' => {
                if self.peeks_ahead(5) == "OTE: " {
                    self.current += 5;
                    self.add_token(TokenType::Note, true, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'T' => {
                if self.peeks_ahead(4) == "IP: " {
                    self.current += 4;
                    self.add_token(TokenType::Tip, true, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'I' => {
                if self.peeks_ahead(10) == "MPORTANT: " {
                    self.current += 10;
                    self.add_token(TokenType::Important, true, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'C' => {
                if self.peeks_ahead(8) == "AUTION: " {
                    self.current += 8;
                    self.add_token(TokenType::Caution, true, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'W' => {
                if self.peeks_ahead(8) == "ARNING: " {
                    self.current += 8;
                    self.add_token(TokenType::Warning, true, 0)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            // Assume that these are macro closes; the parser can always reject it
            ']' => self.add_token(TokenType::InlineMacroClose, true, 0),
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

        // save an allocation for the line bumping
        if advance_line_after != 0 {
            let token_line = self.line;
            self.line += advance_line_after;
            self.startcol = 1;
            Token {
                token_type,
                lexeme: text.to_string(),
                literal,
                line: token_line,
                startcol: token_start,
                endcol: token_start + text.len() - 1, // to account for the start char
            }
        } else {
            self.startcol = token_start + text.len();
            Token {
                token_type,
                lexeme: text.to_string(),
                literal,
                line: self.line,
                startcol: token_start,
                endcol: token_start + text.len() - 1, // to account for the start char
            }
        }
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
            _ => panic!("Too many headings"), // TODO
        }
    }

    // adds the list item token, then includes the rest of the list item (until a new block or
    // another list item marker) in an Text
    fn add_list_item(&mut self, list_item_token: TokenType) -> Token {
        self.current += 1; // advance past the space, which we'll include in the token lexeme
        self.add_token(list_item_token, false, 0)
    }

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

    fn add_text_until_next_markup(&mut self) -> Token {
        // "inline" chars that could be markup; the newline condition prevents
        // capturing significant block markup chars
        // Chars: newline, bold, italic, code, super, subscript, footnote, pass, link, end inline macro, definition list marker, highlighted, inline admonition initial chars
        while !['\n', '*', '_', '`', '^', '~', 'f', 'p', 'h', ']', '[', ':', '#', 'N', 'T', 'I', 'C',
            'W']
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
            && self.source[self.start..self.current + delimiter_len]
                == expected_block
    }

    /// Checks for lines such as [quote], [verse, Mary Oliver], [source, python], etc.
    fn starts_attribute_line(&mut self) -> bool {
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

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        self.source.as_bytes()[self.current] as char
    }

    fn peek_back(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        self.source.as_bytes()[self.start - 1] as char
    }

    fn peeks_ahead(&self, count: usize) -> &str {
        if self.is_at_end() || self.current + count > self.source.len() {
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

    #[test]
    fn newline() {
        let markup = "\n".to_string();
        let expected_tokens = vec![Token::new(
            TokenType::NewLineChar,
            "\n".to_string(),
            None,
            1,
            1,
            1,
        )];
        scan_and_assert_eq(&markup, expected_tokens)
    }

    #[rstest]
    #[case("++++\n".to_string(), TokenType::PassthroughBlock)]
    #[case("****\n".to_string(), TokenType::AsideBlock)]
    #[case("----\n".to_string(), TokenType::SourceBlock)]
    #[case("____\n".to_string(), TokenType::QuoteVerseBlock)]
    #[case("____\n".to_string(), TokenType::QuoteVerseBlock)]
    #[case("====\n".to_string(), TokenType::AdmonitionBlock)]
    fn fenced_block_delimiter_start(#[case] markup: String, #[case] expected_token: TokenType) {
        let expected_tokens = vec![
            Token::new(
                expected_token,
                markup.clone()[..4].to_string(),
                None,
                1,
                1,
                4,
            ),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1, 5, 5),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case("\n\n++++\n".to_string(), TokenType::PassthroughBlock)]
    #[case("\n\n****\n".to_string(), TokenType::AsideBlock)]
    #[case("\n\n----\n".to_string(), TokenType::SourceBlock)]
    #[case("\n\n____\n".to_string(), TokenType::QuoteVerseBlock)]
    #[case("\n\n////\n".to_string(), TokenType::CommentBlock)]
    #[case("\n\n====\n".to_string(), TokenType::AdmonitionBlock)]
    fn fenced_block_delimiter_new_block(#[case] markup: String, #[case] expected_token: TokenType) {
        let expected_tokens = vec![
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2, 1, 1),
            Token::new(
                expected_token,
                markup.clone()[2..6].to_string(),
                None,
                3,
                1,
                4,
            ),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 3, 5, 5),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn open_block_beginning() {
        let markup = "--\n".to_string();
        let expected_tokens = vec![
            Token::new(
                TokenType::OpenBlock,
                markup.clone()[..2].to_string(),
                None,
                1,
                1,
                2,
            ),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1, 3, 3),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn open_block_new_block() {
        let markup = "\n\n--\n".to_string();
        let expected_tokens = vec![
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2, 1, 1),
            Token::new(TokenType::OpenBlock, "--".to_string(), None, 3, 1, 2),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 3, 3, 3),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case("* Foo\n".to_string(), TokenType::UnorderedListItem)]
    #[case(". Foo\n".to_string(), TokenType::OrderedListItem)]
    fn list_items(#[case] markup: String, #[case] expected_token: TokenType) {
        let mut delimiter = markup
            .clone()
            .split_whitespace()
            .next()
            .unwrap()
            .to_string();
        delimiter.push(' ');
        let expected_tokens = vec![
            Token::new(expected_token, delimiter, None, 1, 1, 2),
            Token::new(
                TokenType::Text,
                "Foo".to_string(),
                Some("Foo".to_string()),
                1,
                3,
                5,
            ),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1, 6, 6),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn block_introduction() {
        let markup = ".Some title\n".to_string();
        let title = "Some title".to_string();
        let expected_tokens = vec![
            Token::new(TokenType::BlockLabel, ".".to_string(), None, 1, 1, 1),
            Token::new(TokenType::Text, title.clone(), Some(title), 1, 2, 11),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1, 12, 12),
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
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2, 1, 1),
            Token::new(expected_token, lexeme.clone(), None, 3, 1, lexeme.len()),
            Token::new(
                TokenType::Text,
                "Foo".to_string(),
                Some("Foo".to_string()),
                3,
                lexeme.len() + 1,
                lexeme.len() + 3,
            ),
            Token::new(
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
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2, 1, 1),
            Token::new(
                expected_token,
                markup.clone()[2..5].to_string(),
                None,
                3,
                1,
                3,
            ),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 3, 4, 4),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn comments() {
        let comment_line = "// Some text or other".to_string();
        let markup = "\n".to_string() + &comment_line + "\n";
        let expected_tokens = vec![
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new(
                TokenType::Comment,
                comment_line.clone(),
                Some(comment_line.clone()),
                2,
                1,
                comment_line.len(),
            ),
            Token::new(
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
    #[case("[quote]\n", TokenType::Blockquote)]
    #[case("[quote, Georges Perec]\n", TokenType::Blockquote)]
    #[case("[verse]\n", TokenType::Verse)]
    #[case("[verse, Audre Lorde, A Litany for Survival]\n", TokenType::Verse)]
    #[case("[source]\n", TokenType::Source)]
    #[case("[NOTE]\n", TokenType::Note)]
    #[case("[TIP]\n", TokenType::Tip)]
    #[case("[IMPORTANT]\n", TokenType::Important)]
    #[case("[CAUTION]\n", TokenType::Caution)]
    #[case("[WARNING]\n", TokenType::Warning)]
    fn attribute_lines(#[case] markup: &str, #[case] expected_token: TokenType) {
        let markup_len = markup[..markup.len() - 1].len();
        let expected_tokens = vec![
            Token::new(
                expected_token,
                markup[..markup.len() - 1].to_string(),
                Some(markup[..markup.len() - 1].to_string()),
                1,
                1,
                markup_len,
            ),
            Token::new(
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
            Token::new(
                TokenType::UnorderedListItem,
                "* ".to_string(),
                None,
                1,
                1,
                2,
            ),
            Token::new(
                TokenType::Text,
                "Foo".to_string(),
                Some("Foo".to_string()),
                1,
                3,
                5,
            ),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1, 6, 6),
            Token::new(TokenType::BlockContinuation, "+".to_string(), None, 2, 1, 1),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2, 2, 2),
            Token::new(
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

    #[rstest]
    #[case('*', TokenType::Bold)]
    #[case('_', TokenType::Italic)]
    #[case('`', TokenType::Monospace)]
    #[case('^', TokenType::Superscript)]
    #[case('~', TokenType::Subscript)]
    #[case('#', TokenType::Highlighted)]
    fn inline_formatting(#[case] markup_char: char, #[case] expected_token: TokenType) {
        let markup = format!("Some {}bar{} bar.", markup_char, markup_char);
        let expected_tokens = vec![
            Token::new(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
                1,
                1,
                5,
            ),
            Token::new(expected_token, markup_char.to_string(), None, 1, 6, 6),
            Token::new(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                7,
                9,
            ),
            Token::new(expected_token, markup_char.to_string(), None, 1, 10, 10),
            Token::new(
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
    #[case('*', TokenType::Bold)]
    #[case('_', TokenType::Italic)]
    #[case('`', TokenType::Monospace)]
    #[case('^', TokenType::Superscript)]
    #[case('~', TokenType::Subscript)]
    #[case('#', TokenType::Highlighted)]
    fn inline_formatting_doubles(#[case] markup_char: char, #[case] expected_token: TokenType) {
        // TODO make this less ugly
        let markup = format!(
            "Some {}{}bar{}{} bar.",
            markup_char, markup_char, markup_char, markup_char
        );
        let expected_tokens = vec![
            Token::new(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
                1,
                1,
                5,
            ),
            Token::new(expected_token, markup_char.to_string(), None, 1, 6, 6),
            Token::new(expected_token, markup_char.to_string(), None, 1, 7, 7),
            Token::new(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                8,
                10,
            ),
            Token::new(expected_token, markup_char.to_string(), None, 1, 11, 11),
            Token::new(expected_token, markup_char.to_string(), None, 1, 12, 12),
            Token::new(
                TokenType::Text,
                " bar.".to_string(),
                Some(" bar.".to_string()),
                1,
                13,
                17,
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
            Token::new(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
                1,
                1,
                5,
            ),
            Token::new(
                expected_token,
                markup_check.to_string(),
                None,
                1,
                6,
                6 + markup_check.len() - 1,
            ),
            Token::new(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                6 + markup_check.len(),
                6 + markup_check.len() + 2,
            ),
            Token::new(
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
    #[case("NOTE", TokenType::Note)]
    #[case("TIP", TokenType::Tip)]
    #[case("IMPORTANT", TokenType::Important)]
    #[case("CAUTION", TokenType::Caution)]
    #[case("WARNING", TokenType::Warning)]
    fn inline_admonitions(#[case] markup_check: &str, #[case] expected_token: TokenType) {
        let markup = format!("{}: bar.", markup_check);
        let expected_tokens = vec![
            Token::new(
                expected_token,
                format!("{}: ", markup_check),
                Some(format!("{}: ", markup_check)),
                1,
                1,
                markup_check.len() + 2, // account for space
            ),
            Token::new(
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
    fn definition_list_mark() {
        let markup = "Term:: Definition";
        let expected_tokens = vec![
            Token::new(
                TokenType::Text,
                "Term".to_string(),
                Some("Term".to_string()),
                1,
                1,
                4,
            ),
            Token::new(TokenType::DefListMark, ":: ".to_string(), None, 1, 5, 7),
            Token::new(
                TokenType::Text,
                "De".to_string(),
                Some("De".to_string()),
                1,
                8,
                9,
            ),
            Token::new(
                TokenType::Text,
                "finition".to_string(),
                Some("finition".to_string()),
                1,
                10,
                17,
            ),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }
    #[test]
    fn definition_list_mark_not_if_space_before() {
        let markup = "Term :: bar";

        let expected_tokens = vec![
            Token::new(
                TokenType::Text,
                "Term ".to_string(),
                Some("Term ".to_string()),
                1,
                1,
                5,
            ),
            Token::new(
                TokenType::Text,
                ":".to_string(),
                Some(":".to_string()),
                1,
                6,
                6,
            ),
            Token::new(
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
            Token::new(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
                1,
                1,
                5,
            ),
            Token::new(
                TokenType::LinkMacro,
                "http://example.com[".to_string(),
                Some("http://example.com[".to_string()),
                1,
                6,
                24,
            ),
            Token::new(
                TokenType::Text,
                "bar".to_string(),
                Some("bar".to_string()),
                1,
                25,
                27,
            ),
            Token::new(
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
            Token::new(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
                1,
                1,
                5,
            ),
            Token::new(
                TokenType::LinkMacro,
                "http://example.com[".to_string(),
                Some("http://example.com[".to_string()),
                1,
                6,
                24,
            ),
            Token::new(
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
    fn inline_style_in_line() {
        let markup = "Some [.style]#text#";
        let expected_tokens = vec![
            Token::new(
                TokenType::Text,
                "Some ".to_string(),
                Some("Some ".to_string()),
                1,
                1,
                5,
            ),
            Token::new(
                TokenType::InlineStyle,
                "[.style]".to_string(),
                Some("[.style]".to_string()),
                1,
                6,
                13,
            ),
            Token::new(TokenType::Highlighted, "#".to_string(), None, 1, 14, 14),
            Token::new(
                TokenType::Text,
                "text".to_string(),
                Some("text".to_string()),
                1,
                15,
                18,
            ),
            Token::new(TokenType::Highlighted, "#".to_string(), None, 1, 19, 19),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn inline_style_new_line() {
        let markup = "\n[.style]#text#";
        let expected_tokens = vec![
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new(
                TokenType::InlineStyle,
                "[.style]".to_string(),
                Some("[.style]".to_string()),
                2,
                1,
                8,
            ),
            Token::new(TokenType::Highlighted, "#".to_string(), None, 2, 9, 9),
            Token::new(
                TokenType::Text,
                "text".to_string(),
                Some("text".to_string()),
                2,
                10,
                13,
            ),
            Token::new(TokenType::Highlighted, "#".to_string(), None, 2, 14, 14),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }
    #[test]
    fn inline_style_new_block() {
        let markup = "\n\n[.style]#text#";
        let expected_tokens = vec![
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1, 1, 1),
            Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2, 1, 1),
            Token::new(
                TokenType::InlineStyle,
                "[.style]".to_string(),
                Some("[.style]".to_string()),
                3,
                1,
                8,
            ),
            Token::new(TokenType::Highlighted, "#".to_string(), None, 3, 9, 9),
            Token::new(
                TokenType::Text,
                "text".to_string(),
                Some("text".to_string()),
                3,
                10,
                13,
            ),
            Token::new(TokenType::Highlighted, "#".to_string(), None, 3, 14, 14),
        ];
        scan_and_assert_eq(&markup, expected_tokens);
    }
}
