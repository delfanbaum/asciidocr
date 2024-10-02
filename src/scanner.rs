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
            self.scan_blocks();
        }

        // end of file
        self.tokens
            .push(Token::new(TokenType::Eof, String::new(), None, self.line));

        self.tokens.clone()
    }

    /// First pass, scans for block tokens
    fn scan_blocks(&mut self) {
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
                    self.add_unprocesed_line()
                }
            }
            '<' => {
                if self.starts_repeated_char_line(c, 3) {
                    self.current += 3;
                    self.add_token(TokenType::PageBreak, false);
                    self.line += 1;
                } else {
                    self.add_unprocesed_line()
                }
            }
            // potential block delimiter chars get treated similarly
            '+' | '*' | '-' | '_' | '/' => {
                if self.starts_new_block() && self.starts_repeated_char_line(c, 4) {
                    self.current += 4; // the remaining repeated chars and a newline
                    self.add_token(TokenType::block_from_char(c), false);
                    self.line += 1; // since we consume the newline as a part of the block
                } else {
                    println!("Else: {c}");
                    match c {
                        '-' => {
                            // check if it's an open block
                            println!("checks ");
                            if self.starts_new_block() && self.starts_repeated_char_line(c, 2) {
                                self.current += 2;
                                self.add_token(TokenType::OpenBlock, false);
                                self.line += 1; // since we consume the newline as a part of the block
                            } else {
                                self.add_rest_as_unprocesed_line()
                            }
                        }
                        '*' => {
                            // check if it's a list item
                            if self.starts_new_line() && self.peek() == ' ' {
                                self.add_list_item(TokenType::UnorderedListItem)
                            } else {
                                self.add_rest_as_unprocesed_line()
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
                                self.add_rest_as_unprocesed_line()
                            }
                        }
                        _ => self.add_rest_as_unprocesed_line(),
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
                        self.add_rest_as_unprocesed_line();
                    }
                } else {
                    self.add_unprocesed_line()
                }
            }

            '=' => {
                // possible heading
                if self.starts_new_block() {
                    self.add_heading()
                } else {
                    self.add_unprocesed_line()
                }
            }

            '[' => {
                // role, quote, verse, source, etc
                if self.starts_attribute_line() {
                    println!("{}", &self.source[self.start..self.current]);
                    match self.source.as_bytes()[self.start + 1] as char {
                        'q' => self.add_token(TokenType::Blockquote, true),
                        'v' => self.add_token(TokenType::Verse, true),
                        's' => self.add_token(TokenType::Source, true),
                        _ => panic!("Unexpected attr line"), // TODO
                    }
                } else {
                    println!("{}", &self.source[self.start..self.current]);
                    self.add_unprocesed_line()
                }
            }

            // if it doesn't look like a block thing, save it for future processing
            _ => self.add_unprocesed_line(), // TK
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
        self.add_rest_as_unprocesed_line()
    }

    // adds the list item token, then includes the rest of the list item (until a new block or
    // another list item marker) in an UnprocessedLine
    fn add_list_item(&mut self, list_item_token: TokenType) {
        self.current += 1; // advance past the space, which we'll include in the token literal
        self.add_token(list_item_token, false);
        // include the rest of the line
        self.add_rest_as_unprocesed_line();
    }

    fn add_unprocesed_line(&mut self) {
        while self.peek() != '\n' && !self.is_at_end() {
            self.current += 1;
        }
        self.add_token(TokenType::UnprocessedLine, true)
    }

    fn add_rest_as_unprocesed_line(&mut self) {
        self.start = self.current;
        while self.peek() != '\n' && !self.is_at_end() {
            self.current += 1;
        }
        self.add_token(TokenType::UnprocessedLine, true)
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn starts_new_block(&self) -> bool {
        println!("{:?}", self);
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
        if self.source.as_bytes()[self.current - 1] as char == ']' {
            // i.e., the end of an attribute list line
            true
        } else {
            self.current = current_placeholder;
            false
        }
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        self.source.as_bytes()[self.current] as char
    }
}

#[cfg(test)]
mod tests {
    use crate::tokens::TokenType;
    use rstest::rstest;

    use super::*;

    // utility to make comparison easier given the EOF token
    fn expected_from(mut tokens: Vec<Token>, markup: &str) -> Vec<Token> {
        let last_line = markup.split('\n').count();
        tokens.push(Token::new(TokenType::Eof, String::new(), None, last_line));
        tokens
    }

    #[test]
    fn test_newline() {
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
        let mut s = Scanner::new(&markup);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }

    #[rstest]
    #[case("++++\n".to_string(), TokenType::PassthroughBlock)]
    #[case("****\n".to_string(), TokenType::AsideBlock)]
    #[case("----\n".to_string(), TokenType::SourceBlock)]
    #[case("____\n".to_string(), TokenType::QuoteVerseBlock)]
    #[case("____\n".to_string(), TokenType::QuoteVerseBlock)]
    #[case("--\n".to_string(), TokenType::OpenBlock)]
    fn test_fenced_block_delimiter_start(
        #[case] markup: String,
        #[case] expected_token: TokenType,
    ) {
        let expected_tokens = expected_from(
            vec![Token::new(expected_token, markup.clone(), None, 1)],
            &markup,
        );
        let mut s = Scanner::new(&markup);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }

    #[rstest]
    #[case("\n\n++++\n".to_string(), TokenType::PassthroughBlock)]
    #[case("\n\n****\n".to_string(), TokenType::AsideBlock)]
    #[case("\n\n----\n".to_string(), TokenType::SourceBlock)]
    #[case("\n\n____\n".to_string(), TokenType::QuoteVerseBlock)]
    #[case("\n\n////\n".to_string(), TokenType::CommentBlock)]
    #[case("\n\n--\n".to_string(), TokenType::OpenBlock)]
    fn test_fenced_block_delimiter_new_block(
        #[case] markup: String,
        #[case] expected_token: TokenType,
    ) {
        let expected_tokens = expected_from(
            vec![
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2),
                Token::new(expected_token, markup.clone()[2..].to_string(), None, 3),
            ],
            &markup,
        );
        let mut s = Scanner::new(&markup);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }

    #[rstest]
    #[case("* Foo\n".to_string(), TokenType::UnorderedListItem)]
    #[case(". Foo\n".to_string(), TokenType::OrderedListItem)]
    fn test_list_items(#[case] markup: String, #[case] expected_token: TokenType) {
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
                    TokenType::UnprocessedLine,
                    "Foo".to_string(),
                    Some("Foo".to_string()),
                    1,
                ),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
            ],
            &markup,
        );
        let mut s = Scanner::new(&markup);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }

    #[test]
    fn test_block_introduction() {
        let markup = ".Some title\n".to_string();
        let title = "Some title".to_string();
        let expected_tokens = expected_from(
            vec![
                Token::new(TokenType::BlockLabel, ".".to_string(), None, 1),
                Token::new(TokenType::UnprocessedLine, title.clone(), Some(title), 1),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
            ],
            &markup,
        );
        let mut s = Scanner::new(&markup);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }

    #[rstest]
    #[case("\n\n= Foo\n".to_string(), TokenType::Heading1, 1)]
    #[case("\n\n== Foo\n".to_string(), TokenType::Heading2, 2)]
    #[case("\n\n=== Foo\n".to_string(), TokenType::Heading3, 3)]
    #[case("\n\n==== Foo\n".to_string(), TokenType::Heading4, 4)]
    #[case("\n\n===== Foo\n".to_string(), TokenType::Heading5, 5)]
    fn test_headings_after_block(
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
                    TokenType::UnprocessedLine,
                    "Foo".to_string(),
                    Some("Foo".to_string()),
                    3,
                ),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 3),
            ],
            &markup,
        );
        let mut s = Scanner::new(&markup);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }

    #[rstest]
    #[case("\n\n'''\n".to_string(), TokenType::ThematicBreak)]
    #[case("\n\n<<<\n".to_string(), TokenType::PageBreak)]
    fn test_breaks(#[case] markup: String, #[case] expected_token: TokenType) {
        // these should always be after a block, and the 'start' case is tested elsewhere
        let expected_tokens = expected_from(
            vec![
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2),
                Token::new(expected_token, markup.clone()[2..].to_string(), None, 3),
            ],
            &markup,
        );
        let mut s = Scanner::new(&markup);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }

    #[test]
    fn test_comments() {
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
        let mut s = Scanner::new(&markup);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }

    #[rstest]
    #[case("[quote]\n", TokenType::Blockquote)]
    #[case("[quote, Georges Perec]\n", TokenType::Blockquote)]
    #[case("[verse]\n", TokenType::Verse)]
    #[case("[verse, Audre Lorde, A Litany for Survival]\n", TokenType::Verse)]
    #[case("[source]\n", TokenType::Source)]
    fn test_attribute_lines(#[case] markup: &str, #[case] expected_token: TokenType) {
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
        let mut s = Scanner::new(&markup);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }

    #[test]
    fn test_block_continuation() {
        let markup = "* Foo\n+\nBar".to_string();
        let expected_tokens = expected_from(
            vec![
                Token::new(TokenType::UnorderedListItem, "* ".to_string(), None, 1),
                Token::new(
                    TokenType::UnprocessedLine,
                    "Foo".to_string(),
                    Some("Foo".to_string()),
                    1,
                ),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 1),
                Token::new(TokenType::BlockContinuation, "+".to_string(), None, 2),
                Token::new(TokenType::NewLineChar, "\n".to_string(), None, 2),
                Token::new(
                    TokenType::UnprocessedLine,
                    "Bar".to_string(),
                    Some("Bar".to_string()),
                    3,
                ),
            ],
            &markup,
        );
        let mut s = Scanner::new(&markup);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }
}
