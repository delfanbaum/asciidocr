use crate::{
    asg::Asg,
    blocks::Block,
    inlines::Inline,
    tokens::{Token, TokenType},
};

pub struct Parser<'a> {
    last_token: Option<&'a Token>,
    open_blocks: Vec<Block>,
    open_inlines: Vec<Inline>,
}

impl<'a> Parser<'a> {
    pub fn new() -> Self {
        Parser {
            last_token: None,
            open_blocks: vec![],
            open_inlines: vec![],
        }
    }

    pub fn parse<I>(&mut self, tokens: I) -> Asg
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

    fn token_into(&mut self, token: Token, asg: &mut Asg) {
        match token.token_type() {
            TokenType::NewLineChar => self.parse_new_line_char(token, asg),
            
            _ => {}
        }
    }

    fn parse_new_line_char(&mut self, token: Token, asg: &mut Asg) {
                if self.last_token_type() == TokenType::NewLineChar {
                    self.open_blocks.pop();
                }
                // ...should we always append a "\n" char to the last string? Probably?
    }

    fn parse_thematic_break(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_page_break(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_comment(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_passthrough_block(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_aside_block(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_source_block(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_quote_verse_block(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_comment_block(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_admonition_block(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_open_block(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_ordered_list_item(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_unordered_list_item(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_block_label(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_heading1(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_heading2(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_heading3(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_heading4(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_heading5(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_blockquote(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_verse(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_source(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_note(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_tip(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_important(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_caution(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_warning(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_block_continuation(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_def_list_mark(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_bold(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_italic(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_monospace(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_superscript(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_subscript(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_highlighted(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_link_macro(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_footnote_macro(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_passthrough_inline_macro(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_inline_macro_close(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_text(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_eof(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_inline_style(&mut self, token: Token, asg: &mut Asg) {}

    fn last_token_type(&self) -> TokenType {
        if self.last_token.is_some() {
            self.last_token.unwrap().token_type()
        } else {
            TokenType::Eof
        }
    }
}
