use crate::tokens::Token;

struct Scanner {
    source: String,
    tokens: Vec<Token>,
}

impl Scanner {
    fn new(s: String) -> Self {
        Scanner {
            source: s,
            tokens: Vec::new(),
        }
    }
}

impl Scanner {
    fn scan(&mut self) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::tokens::TokenType;

    use super::*;

    fn plain_text_string() {
        let test_string = "This should be one text token.\n".to_string();
        let expected_tokens = vec![Token::new(
            TokenType::Text,
            1,
            "This should be one text token\n".to_string(),
            Some("This should be one text token\n".to_string()),
        )];
        let mut s = Scanner::new(test_string);
        s.scan();
        assert_eq!(expected_tokens, s.tokens);
    }
}
