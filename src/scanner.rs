use crate::tokens::{Token, TokenType};

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
            '\n' => self.add_token(TokenType::NewLineChar, None),
            _ => {} // TK
        }
    }

    fn add_token(&mut self, token_type: TokenType, literal: Option<String>) {
        let text = &self.source[self.start..self.current];
        self.tokens.push(Token {
            token_type,
            lexeme: text.to_string(),
            literal,
            line: self.line,
        })
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::tokens::TokenType;

    use super::*;

    // utility to make comparison easier given the EOF token
    fn expected_from(mut tokens: Vec<Token>, last_line: usize) -> Vec<Token> {
        tokens.push(Token::new(TokenType::Eof, String::new(), None, last_line));
        tokens
    }

    #[test]
    fn test_newline() {
        let test_string = "\n".to_string();
        let expected_tokens = expected_from(vec![Token::new(
            TokenType::NewLineChar,
            "\n".to_string(),
            None,
            1,
        )], 1);
        let mut s = Scanner::new(test_string);
        s.scan_tokens();
        assert_eq!(expected_tokens, s.tokens);
    }
}
