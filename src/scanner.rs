use core::{panic, str};

use crate::tokens::{Token, TokenType};

#[derive(Debug)]
pub struct Scanner<'a> {
    pub source: &'a str,
    pub tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        Scanner {
            source,
            tokens: Vec::new(),
            start: 0,   // beginning of the current lexeme being scanned
            current: 0, // the character we're looking at *now*
            line: 1,
        }
    }

    pub fn scan_tokens(&mut self) -> Vec<Token> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        // end of file
        self.tokens
            .push(Token::new(TokenType::Eof, String::new(), None, self.line));

        self.tokens.clone()
    }

    fn scan_token(&mut self) {
        let c = self.source.as_bytes()[self.current] as char;
        self.current += 1; // this instead of the "advance" function in "Crafting Interpreters"

        match c {
            '\n' => {
                self.add_token(TokenType::NewLineChar, false);
                self.line += 1;
            }

            '\'' => {
                if self.starts_repeated_char_line(c, 3) {
                    self.current += 3;
                    self.add_token(TokenType::ThematicBreak, false);
                    self.line += 1;
                } else {
                    self.add_text_until_next_markup()
                }
            }
            '<' => {
                if self.starts_repeated_char_line(c, 3) {
                    self.current += 3;
                    self.add_token(TokenType::PageBreak, false);
                    self.line += 1;
                } else {
                    self.add_text_until_next_markup()
                }
            }
            // potential block delimiter chars get treated similarly
            '+' | '*' | '-' | '_' | '/' | '=' => {
                if self.starts_new_block() && self.starts_repeated_char_line(c, 4) {
                    self.current += 4; // the remaining repeated chars and a newline
                    self.add_token(TokenType::block_from_char(c), false);
                    self.line += 1; // since we consume the newline as a part of the block
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
                                self.current += 2;
                                self.add_token(TokenType::OpenBlock, false);
                                self.line += 1; // since we consume the newline as a part of the block
                            } else {
                                self.add_text_until_next_markup()
                            }
                        }
                        '*' => {
                            // check if it's a list item
                            if self.starts_new_line() && self.peek() == ' ' {
                                self.add_list_item(TokenType::UnorderedListItem)
                            } else {
                                self.add_token(TokenType::Bold, false)
                            }
                        }
                        '/' => {
                            if self.starts_new_line() && self.peek() == '/' {
                                while self.peek() != '\n' && !self.is_at_end() {
                                    self.current += 1;
                                }
                                self.add_token(TokenType::Comment, true)
                            } else {
                            }
                        }
                        '+' => {
                            if self.starts_new_line() && self.peek() == '\n' {
                                self.add_token(TokenType::BlockContinuation, false)
                            } else {
                                self.add_text_until_next_markup()
                            }
                        }
                        '_' => self.add_token(TokenType::Italic, false),
                        _ => self.add_text_until_next_markup(),
                    }
                }
            }

            // ordered list item or section title
            '.' => {
                if self.starts_new_block() {
                    if self.peek() == ' ' {
                        self.add_list_item(TokenType::OrderedListItem);
                    } else {
                        self.add_token(TokenType::BlockLabel, false);
                    }
                } else {
                    self.add_text_until_next_markup()
                }
            }

            '[' => {
                // role, quote, verse, source, etc
                if self.starts_attribute_line() {
                    match self.source.as_bytes()[self.start + 1] as char {
                        'q' => self.add_token(TokenType::Blockquote, true),
                        'v' => self.add_token(TokenType::Verse, true),
                        's' => self.add_token(TokenType::Source, true),
                        'N' => self.add_token(TokenType::Note, true),
                        'T' => self.add_token(TokenType::Tip, true),
                        'I' => self.add_token(TokenType::Important, true),
                        'C' => self.add_token(TokenType::Caution, true),
                        'W' => self.add_token(TokenType::Warning, true),
                        _ => self.add_text_until_next_markup(),
                    }
                } else if self.peek() == '.' {
                    self.add_inline_style()
                } else {
                    self.add_text_until_next_markup()
                }
            }

            '`' => self.add_token(TokenType::Monospace, false),
            '^' => self.add_token(TokenType::Superscript, false),
            '~' => self.add_token(TokenType::Subscript, false),
            '#' => self.add_token(TokenType::Highlighted, false),
            ':' => {
                if self.peek_back() != ' ' && self.peeks_ahead(2) == ": " {
                    self.current += 2;
                    self.add_token(TokenType::DefListMark, false)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'f' => {
                // guard against indexing out of range
                if self.peeks_ahead(9) == "ootnote:[" {
                    self.current += 9;
                    self.add_token(TokenType::FootnoteMacro, false)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'p' => {
                if self.peeks_ahead(5) == "ass:[" {
                    self.current += 5;
                    self.add_token(TokenType::PassthroughInlineMacro, false)
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
                    self.add_token(TokenType::Note, true)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'T' => {
                if self.peeks_ahead(4) == "IP: " {
                    self.current += 4;
                    self.add_token(TokenType::Tip, true)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'I' => {
                if self.peeks_ahead(10) == "MPORTANT: " {
                    self.current += 10;
                    self.add_token(TokenType::Important, true)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'C' => {
                if self.peeks_ahead(8) == "AUTION: " {
                    self.current += 8;
                    self.add_token(TokenType::Caution, true)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            'W' => {
                if self.peeks_ahead(8) == "ARNING: " {
                    self.current += 8;
                    self.add_token(TokenType::Warning, true)
                } else {
                    self.add_text_until_next_markup()
                }
            }
            // Assume that these are macro closes; the parser can always reject it
            ']' => self.add_token(TokenType::InlineMacroClose, true),
            _ => self.add_text_until_next_markup(),
        }
    }

    fn add_token(&mut self, token_type: TokenType, include_literal: bool) {
        let text = &self.source[self.start..self.current];
        let mut literal = None;
        if include_literal {
            literal = Some(text.to_string())
        }
        self.tokens.push(Token {
            token_type,
            lexeme: text.to_string(),
            literal,
            line: self.line,
        })
    }

    fn add_heading(&mut self) {
        while self.peek() == '=' {
            self.current += 1
        }
        self.current += 1; // add the space to the lexeme, but remove from count
        match self.source.as_bytes()[self.start..self.current - 1].len() {
            1 => self.add_token(TokenType::Heading1, false),
            2 => self.add_token(TokenType::Heading2, false),
            3 => self.add_token(TokenType::Heading3, false),
            4 => self.add_token(TokenType::Heading4, false),
            5 => self.add_token(TokenType::Heading5, false),
            _ => panic!("Too many headings"), // TODO
        }
    }

    // adds the list item token, then includes the rest of the list item (until a new block or
    // another list item marker) in an Text
    fn add_list_item(&mut self, list_item_token: TokenType) {
        self.current += 1; // advance past the space, which we'll include in the token lexeme
        self.add_token(list_item_token, false);
    }

    fn add_link(&mut self) {
        while self.peek() != '[' {
            self.current += 1
        }
        self.current += 1; // consume the '[' char
        self.add_token(TokenType::LinkMacro, true)
    }

    fn add_inline_style(&mut self) {
        while self.peek() != ']' {
            self.current += 1
        }
        self.current += 1; // consume the ']' char
        self.add_token(TokenType::InlineStyle, true)
    }

    fn add_text_until_next_markup(&mut self) {
        // "inline" chars that could be markup; the newline condition prevents
        // capturing significant block markup chars
        // Chars: newline, bold, italic, code, super, subscript, footnote, pass, link, end inline macro, definition list marker, highlighted, inline admonition initial chars
        while !vec![
            '\n', '*', '_', '`', '^', '~', 'f', 'p', 'h', ']', '[', ':', '#', 'N', 'T', 'I', 'C',
            'W',
        ]
        .contains(&self.peek())
            && !self.is_at_end()
        {
            self.current += 1;
        }
        self.add_token(TokenType::Text, true)
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
            && str::from_utf8(&self.source.as_bytes()[self.start..self.current + delimiter_len])
                .unwrap()
                == &expected_block
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

    // utility to make comparison easier given the EOF token
    fn expected_from(mut tokens: Vec<Token>, markup: &str) -> Vec<Token> {
        let last_line = markup.split('\n').count();
        tokens.push(Token::new(TokenType::Eof, String::new(), None, last_line));
        tokens
    }

    fn scan_and_assert_eq(markup: &str, expected_tokens: Vec<Token>) {
        let mut s = Scanner::new(markup);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }

    #[test]
    fn newline() {
        let markup = "\n".to_string();
        let expected_tokens = expected_from(
            vec![Token::new(
                TokenType::NewLineChar,
                "\n".to_string(),
                None,
                1,
            )],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens)
    }

    #[rstest]
    #[case("++++\n".to_string(), TokenType::PassthroughBlock)]
    #[case("****\n".to_string(), TokenType::AsideBlock)]
    #[case("----\n".to_string(), TokenType::SourceBlock)]
    #[case("____\n".to_string(), TokenType::QuoteVerseBlock)]
    #[case("____\n".to_string(), TokenType::QuoteVerseBlock)]
    #[case("====\n".to_string(), TokenType::AdmonitionBlock)]
    #[case("--\n".to_string(), TokenType::OpenBlock)]
    fn fenced_block_delimiter_start(#[case] markup: String, #[case] expected_token: TokenType) {
        let expected_tokens = expected_from(
            vec![Token::new(expected_token, markup.clone(), None, 1)],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case("\n\n++++\n".to_string(), TokenType::PassthroughBlock)]
    #[case("\n\n****\n".to_string(), TokenType::AsideBlock)]
    #[case("\n\n----\n".to_string(), TokenType::SourceBlock)]
    #[case("\n\n____\n".to_string(), TokenType::QuoteVerseBlock)]
    #[case("\n\n////\n".to_string(), TokenType::CommentBlock)]
    #[case("\n\n====\n".to_string(), TokenType::AdmonitionBlock)]
    #[case("\n\n--\n".to_string(), TokenType::OpenBlock)]
    fn fenced_block_delimiter_new_block(#[case] markup: String, #[case] expected_token: TokenType) {
        let expected_tokens = expected_from(
            vec![
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2),
                Token::new(expected_token, markup.clone()[2..].to_string(), None, 3),
            ],
            &markup,
        );
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
        let expected_tokens = expected_from(
            vec![
                Token::new(expected_token, delimiter, None, 1),
                Token::new(
                    TokenType::Text,
                    "Foo".to_string(),
                    Some("Foo".to_string()),
                    1,
                ),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn block_introduction() {
        let markup = ".Some title\n".to_string();
        let title = "Some title".to_string();
        let expected_tokens = expected_from(
            vec![
                Token::new(TokenType::BlockLabel, ".".to_string(), None, 1),
                Token::new(TokenType::Text, title.clone(), Some(title), 1),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
            ],
            &markup,
        );
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
        let expected_tokens = expected_from(
            vec![
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2),
                Token::new(expected_token, lexeme, None, 3),
                Token::new(
                    TokenType::Text,
                    "Foo".to_string(),
                    Some("Foo".to_string()),
                    3,
                ),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 3),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case("\n\n'''\n".to_string(), TokenType::ThematicBreak)]
    #[case("\n\n<<<\n".to_string(), TokenType::PageBreak)]
    fn breaks(#[case] markup: String, #[case] expected_token: TokenType) {
        // these should always be after a block, and the 'start' case is tested elsewhere
        let expected_tokens = expected_from(
            vec![
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2),
                Token::new(expected_token, markup.clone()[2..].to_string(), None, 3),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn comments() {
        let comment_line = "// Some text or other".to_string();
        let markup = "\n".to_string() + &comment_line + "\n";
        let expected_tokens = expected_from(
            vec![
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
                Token::new(
                    TokenType::Comment,
                    comment_line.clone(),
                    Some(comment_line),
                    2,
                ),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2),
            ],
            &markup,
        );
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
        let expected_tokens = expected_from(
            vec![
                Token::new(
                    expected_token,
                    markup[..markup.len() - 1].to_string(),
                    Some(markup[..markup.len() - 1].to_string()),
                    1,
                ),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn block_continuation() {
        let markup = "* Foo\n+\nBar".to_string();
        let expected_tokens = expected_from(
            vec![
                Token::new(TokenType::UnorderedListItem, "* ".to_string(), None, 1),
                Token::new(
                    TokenType::Text,
                    "Foo".to_string(),
                    Some("Foo".to_string()),
                    1,
                ),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
                Token::new(TokenType::BlockContinuation, "+".to_string(), None, 2),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2),
                Token::new(
                    TokenType::Text,
                    "Bar".to_string(),
                    Some("Bar".to_string()),
                    3,
                ),
            ],
            &markup,
        );
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
        let expected_tokens = expected_from(
            vec![
                Token::new(
                    TokenType::Text,
                    "Some ".to_string(),
                    Some("Some ".to_string()),
                    1,
                ),
                Token::new(expected_token, markup_char.to_string(), None, 1),
                Token::new(
                    TokenType::Text,
                    "bar".to_string(),
                    Some("bar".to_string()),
                    1,
                ),
                Token::new(expected_token, markup_char.to_string(), None, 1),
                Token::new(
                    TokenType::Text,
                    " bar.".to_string(),
                    Some(" bar.".to_string()),
                    1,
                ),
            ],
            &markup,
        );
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
        let expected_tokens = expected_from(
            vec![
                Token::new(
                    TokenType::Text,
                    "Some ".to_string(),
                    Some("Some ".to_string()),
                    1,
                ),
                Token::new(expected_token, markup_char.to_string(), None, 1),
                Token::new(expected_token, markup_char.to_string(), None, 1),
                Token::new(
                    TokenType::Text,
                    "bar".to_string(),
                    Some("bar".to_string()),
                    1,
                ),
                Token::new(expected_token, markup_char.to_string(), None, 1),
                Token::new(expected_token, markup_char.to_string(), None, 1),
                Token::new(
                    TokenType::Text,
                    " bar.".to_string(),
                    Some(" bar.".to_string()),
                    1,
                ),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[rstest]
    #[case("footnote:[", TokenType::FootnoteMacro)]
    #[case("pass:[", TokenType::PassthroughInlineMacro)]
    fn inline_macros(#[case] markup_check: &str, #[case] expected_token: TokenType) {
        let markup = format!("Some {}bar]", markup_check);
        let expected_tokens = expected_from(
            vec![
                Token::new(
                    TokenType::Text,
                    "Some ".to_string(),
                    Some("Some ".to_string()),
                    1,
                ),
                Token::new(expected_token, markup_check.to_string(), None, 1),
                Token::new(
                    TokenType::Text,
                    "bar".to_string(),
                    Some("bar".to_string()),
                    1,
                ),
                Token::new(
                    TokenType::InlineMacroClose,
                    "]".to_string(),
                    Some("]".to_string()),
                    1,
                ),
            ],
            &markup,
        );
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
        let expected_tokens = expected_from(
            vec![
                Token::new(
                    expected_token,
                    format!("{}: ", markup_check),
                    Some(format!("{}: ", markup_check)),
                    1,
                ),
                Token::new(
                    TokenType::Text,
                    "bar.".to_string(),
                    Some("bar.".to_string()),
                    1,
                ),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn definition_list_mark() {
        let markup = "Term:: Definition";
        let expected_tokens = expected_from(
            vec![
                Token::new(
                    TokenType::Text,
                    "Term".to_string(),
                    Some("Term".to_string()),
                    1,
                ),
                Token::new(TokenType::DefListMark, ":: ".to_string(), None, 1),
                Token::new(TokenType::Text, "De".to_string(), Some("De".to_string()), 1),
                Token::new(
                    TokenType::Text,
                    "finition".to_string(),
                    Some("finition".to_string()),
                    1,
                ),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }
    #[test]
    fn defintion_list_mark_not_if_space_before() {
        let markup = "Term :: bar";

        let expected_tokens = expected_from(
            vec![
                Token::new(
                    TokenType::Text,
                    "Term ".to_string(),
                    Some("Term ".to_string()),
                    1,
                ),
                Token::new(TokenType::Text, ":".to_string(), Some(":".to_string()), 1),
                Token::new(
                    TokenType::Text,
                    ": bar".to_string(),
                    Some(": bar".to_string()),
                    1,
                ),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn link_with_text() {
        let markup = "Some http://example.com[bar]";
        let expected_tokens = expected_from(
            vec![
                Token::new(
                    TokenType::Text,
                    "Some ".to_string(),
                    Some("Some ".to_string()),
                    1,
                ),
                Token::new(
                    TokenType::LinkMacro,
                    "http://example.com[".to_string(),
                    Some("http://example.com[".to_string()),
                    1,
                ),
                Token::new(
                    TokenType::Text,
                    "bar".to_string(),
                    Some("bar".to_string()),
                    1,
                ),
                Token::new(
                    TokenType::InlineMacroClose,
                    "]".to_string(),
                    Some("]".to_string()),
                    1,
                ),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn link_no_text() {
        let markup = "Some http://example.com[]";
        let expected_tokens = expected_from(
            vec![
                Token::new(
                    TokenType::Text,
                    "Some ".to_string(),
                    Some("Some ".to_string()),
                    1,
                ),
                Token::new(
                    TokenType::LinkMacro,
                    "http://example.com[".to_string(),
                    Some("http://example.com[".to_string()),
                    1,
                ),
                Token::new(
                    TokenType::InlineMacroClose,
                    "]".to_string(),
                    Some("]".to_string()),
                    1,
                ),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn inline_style_in_line() {
        let markup = "Some [.style]#text#";
        let expected_tokens = expected_from(
            vec![
                Token::new(
                    TokenType::Text,
                    "Some ".to_string(),
                    Some("Some ".to_string()),
                    1,
                ),
                Token::new(
                    TokenType::InlineStyle,
                    "[.style]".to_string(),
                    Some("[.style]".to_string()),
                    1,
                ),
                Token::new(TokenType::Highlighted, "#".to_string(), None, 1),
                Token::new(
                    TokenType::Text,
                    "text".to_string(),
                    Some("text".to_string()),
                    1,
                ),
                Token::new(TokenType::Highlighted, "#".to_string(), None, 1),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }

    #[test]
    fn inline_style_new_line() {
        let markup = "\n[.style]#text#";
        let expected_tokens = expected_from(
            vec![
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
                Token::new(
                    TokenType::InlineStyle,
                    "[.style]".to_string(),
                    Some("[.style]".to_string()),
                    2,
                ),
                Token::new(TokenType::Highlighted, "#".to_string(), None, 2),
                Token::new(
                    TokenType::Text,
                    "text".to_string(),
                    Some("text".to_string()),
                    2,
                ),
                Token::new(TokenType::Highlighted, "#".to_string(), None, 2),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }
    #[test]
    fn inline_style_new_block() {
        let markup = "\n\n[.style]#text#";
        let expected_tokens = expected_from(
            vec![
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2),
                Token::new(
                    TokenType::InlineStyle,
                    "[.style]".to_string(),
                    Some("[.style]".to_string()),
                    3,
                ),
                Token::new(TokenType::Highlighted, "#".to_string(), None, 3),
                Token::new(
                    TokenType::Text,
                    "text".to_string(),
                    Some("text".to_string()),
                    3,
                ),
                Token::new(TokenType::Highlighted, "#".to_string(), None, 3),
            ],
            &markup,
        );
        scan_and_assert_eq(&markup, expected_tokens);
    }
}
