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
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_tokens(&mut self) -> Vec<Token> {
        while self.current <= self.source.len() {
            self.start = self.current;
            self.scan_token();
        }

        // end of file
        self.tokens
            .push(Token::new(TokenType::Eof, self.line, String::new(), None));

        self.tokens.clone()
    }

    fn scan_token(&mut self) {
        todo!()
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
