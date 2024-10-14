use crate::{asg::Asg, tokens::Token};

pub struct Parser {}

impl Parser {
    pub fn new() -> Self {
        Parser {}
    }

    pub fn parse<'a, I>(&mut self, tokens: I) -> Asg
    where
        I: Iterator<Item = Token>,
    {
        let mut asg = Asg::new();
        for token in tokens {
            self.token_into(token, &mut asg)
        }
        // TODO get last location in tree
        asg
    }

    fn token_into(&mut self, _token: Token, _asg: &mut Asg) {}
}
