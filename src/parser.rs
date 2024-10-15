use core::panic;

use crate::{
    asg::Asg,
    blocks::{Block, Break, LeafBlock},
    inlines::{Inline, InlineLiteral},
    tokens::{Token, TokenType},
};

pub struct Parser {
    last_token_type: TokenType,
    open_blocks: Vec<Block>,
    //open_inlines: Vec<Inline>,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            last_token_type: TokenType::Eof,
            open_blocks: vec![],
            //open_inlines: vec![],
        }
    }

    pub fn parse<I>(&mut self, tokens: I) -> Asg
    where
        I: Iterator<Item = Token>,
    {
        let mut asg = Asg::new();
        for token in tokens {
            self.last_token_type = token.token_type();
            self.token_into(token, &mut asg)
        }
        // TODO get last location in tree -- and other cleanup, probably an asg.consolidate() or
        // something
        asg.consolidate();
        asg
    }

    fn token_into(&mut self, token: Token, asg: &mut Asg) {
        // handle being inside a PassthroughBlock
        if self.last_token_type == TokenType::PassthroughBlock {
            if token.token_type() != TokenType::PassthroughBlock {
                // if it's not a close delimiter, just add it as literal text
                self.add_inline_to_block_stack(Inline::InlineLiteral(
                    InlineLiteral::new_text_from_token(&token),
                ));
                return; // Break out of the loop
            }
        }

        match token.token_type() {
            TokenType::NewLineChar => self.parse_new_line_char(token, asg),
            TokenType::PageBreak => self.parse_page_break(token, asg),
            TokenType::ThematicBreak => self.parse_thematic_break(token, asg),
            TokenType::PassthroughBlock => self.parse_passthrough_block(token, asg),
            TokenType::SourceBlock => self.parse_source_block(token, asg),
            TokenType::Text => self.parse_text(token),
            TokenType::Comment => self.parse_comment(),
            _ => {}
        }
    }

    fn parse_new_line_char(&mut self, token: Token, asg: &mut Asg) {
        if self.last_token_type == TokenType::NewLineChar {
            if let Some(last_block) = self.open_blocks.pop() {
                self.add_to_block_stack_or_graph(asg, last_block)
            } else {
                // add newline literal --> creates a paragraph on the block stack
                self.add_inline_to_block_stack(Inline::InlineLiteral(
                    InlineLiteral::new_text_from_token(&token),
                ))
            }
        } // else add a newline char
    }

    fn parse_thematic_break(&mut self, token: Token, asg: &mut Asg) {
        self.add_to_block_stack_or_graph(
            asg,
            Block::Break(Break::new(
                crate::blocks::BreakVariant::Thematic,
                token.locations(),
            )),
        )
    }

    fn parse_page_break(&mut self, token: Token, asg: &mut Asg) {
        self.add_to_block_stack_or_graph(
            asg,
            Block::Break(Break::new(
                crate::blocks::BreakVariant::Page,
                token.locations(),
            )),
        )
    }

    fn parse_comment(&self) {
        // for now, do nothing
    }

    fn parse_passthrough_block(&mut self, token: Token, asg: &mut Asg) {
        let block = LeafBlock::new_pass(Some(token.text()), token.first_location());
        self.handle_delimited_leaf_block(asg, block)
    }

    fn parse_source_block(&mut self, token: Token, asg: &mut Asg) {
        let block = LeafBlock::new_listing(Some(token.text()), token.first_location());
        self.handle_delimited_leaf_block(asg, block)
    }

    //fn parse_aside_block(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_quote_verse_block(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_comment_block(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_admonition_block(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_open_block(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_ordered_list_item(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_unordered_list_item(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_block_label(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_heading1(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_heading2(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_heading3(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_heading4(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_heading5(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_blockquote(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_verse(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_source(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_note(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_tip(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_important(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_caution(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_warning(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_block_continuation(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_def_list_mark(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_bold(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_italic(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_monospace(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_superscript(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_subscript(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_highlighted(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_link_macro(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_footnote_macro(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_passthrough_inline_macro(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_inline_macro_close(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_eof(&mut self, token: Token, asg: &mut Asg) {}
    //fn parse_inline_style(&mut self, token: Token, asg: &mut Asg) {}

    fn parse_text(&mut self, token: Token) {
        self.add_inline_to_block_stack(Inline::InlineLiteral(InlineLiteral::new_text_from_token(
            &token,
        )))
    }

    fn handle_delimited_leaf_block(&mut self, asg: &mut Asg, block: LeafBlock) {
        if let Some(open_block) = self.open_blocks.last_mut() {
            match open_block {
                Block::LeafBlock(leaf) => {
                    if leaf.name == block.name {
                        self.close_last_open_block(asg)
                    } else {
                        self.add_to_block_stack_or_graph(asg, Block::LeafBlock(block))
                    }
                }
                _ => self.open_blocks.push(Block::LeafBlock(block)),
            }
        }
    }

    fn close_last_open_block(&mut self, asg: &mut Asg) {
        if let Some(block) = self.open_blocks.pop() {
            self.add_to_block_stack_or_graph(asg, block)
        } else {
            panic!("Unexpected close last block call!")
        }
    }

    fn add_inline_to_block_stack(&mut self, inline: Inline) {
        if let Some(parent_block) = self.open_blocks.last_mut() {
            parent_block.push_inline(inline)
        } else {
            self.open_blocks.push(Block::LeafBlock(LeafBlock::new(
                crate::blocks::LeafBlockName::Paragraph,
                crate::blocks::LeafBlockForm::Paragraph,
                None,
                inline.locations(),
                vec![inline],
            )))
        }
    }

    fn add_to_block_stack_or_graph(&mut self, asg: &mut Asg, block: Block) {
        if let Some(parent_block) = self.open_blocks.last_mut() {
            parent_block.push_block(block)
        } else {
            asg.push_block(block)
        }
    }
}
