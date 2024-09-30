use core::str;

use crate::tokens::{self, Token, TokenType};

pub struct Scanner {
    pub source: String,
    pub tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
}

impl Scanner {
    pub fn new(source: String) -> Self {
        Scanner {
            source,
            tokens: Vec::new(),
            start: 0,
            current: 0,
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
            .push(Token::new(TokenType::Eof, self.line, String::new(), None));

        self.tokens.clone()
    }

    fn scan_token(&mut self) {
        let c = self.source.as_bytes()[self.current] as char;
        self.current += 1;

        match c {
            '*' => {
                if self.starts_delimiter(&c) {
                    self.current += 5; // this is fucked
                    self.add_token(TokenType::AsideBlock, None);
                }
            }
            _ => todo!(),
        }
    }

    fn add_token(&mut self, token_type: TokenType, literal: Literal) {
        let text = &self.source[self.start..self.current];
        self.tokens.push(Token {
            token_type,
            lexeme: text.to_string(),
            literal,
            line: self.line,
        })
    }

    fn is_at_end(self) -> bool {
        if self.current <= self.source.len() {
            false
        } else {
            true
        }
    }

    fn peek(self) -> Option<char> {
        if self.is_at_end() {
            ()
        }
        Some(self.source.as_bytes()[self.current] as char)
    }

    /// check to see if a given character starts a delimiter line, such as "****\n"
    fn starts_delimiter(self, char_to_check: char) -> bool {
        todo!()
    }

    /// check to see if a given character starts a list item line
    fn starts_list_item(&mut self) -> bool {
        if let Some(token) = self.tokens.last() {
            if token.token_type == TokenType::NewLineChar && self.peek() {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::tokens::TokenType;

    use super::*;

    #[test]
    fn test_asideblock() {
        let test_string = "****\n".to_string();
        let expected_tokens = vec![Token::new(
            TokenType::AsideBlock,
            1,
            "*****\n".to_string(),
            None,
        )];
        let mut s = Scanner::new(test_string);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }

    #[test]
    fn plain_text_string() {
        let test_string = "This should be one text token.\n".to_string();
        let expected_tokens = vec![Token::new(
            TokenType::Text,
            1,
            "This should be one text token\n".to_string(),
            Some("This should be one text token\n".to_string()),
        )];
        let mut s = Scanner::new(test_string);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }
}
