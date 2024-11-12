use crate::nodes::Location;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub token_type: TokenType,
    //id: Option<String>,
    pub lexeme: String,          // raw string of code
    pub literal: Option<String>, // our literals are only ever strings (or represented as such)
    pub line: usize,
    pub startcol: usize,
    pub endcol: usize,
}

impl Default for Token {
    fn default() -> Self {
        // defaults to EOF
        Token {
            token_type: TokenType::Eof,
            lexeme: "\0".to_string(),
            literal: None,
            line: 0,
            startcol: 1,
            endcol: 1,
        }
    }
}

impl Token {
    pub fn new(
        token_type: TokenType,
        lexeme: String,
        literal: Option<String>,
        line: usize,
        startcol: usize,
        endcol: usize,
    ) -> Self {
        Token {
            token_type,
            lexeme,
            literal,
            line,
            startcol,
            endcol,
        }
    }

    pub fn token_type(&self) -> TokenType {
        self.token_type
    }

    pub fn text(&self) -> String {
        if let Some(text) = &self.literal {
            text.clone()
        } else {
            self.lexeme.clone()
        }
    }

    pub fn first_location(&self) -> Location {
        Location::new(self.line, self.startcol)
    }
    pub fn last_location(&self) -> Location {
        Location::new(self.line, self.endcol)
    }
    pub fn locations(&self) -> Vec<Location> {
        vec![self.first_location(), self.last_location()]
    }

    pub fn is_inline(&self) -> bool {
        matches!(self.token_type(), |TokenType::Comment| TokenType::Text
            | TokenType::Emphasis
            | TokenType::Strong
            | TokenType::Monospace
            | TokenType::Mark
            | TokenType::NewLineChar
            | TokenType::UnconstrainedEmphasis
            | TokenType::UnconstrainedStrong
            | TokenType::UnconstrainedMonospace
            | TokenType::UnconstrainedMark
            | TokenType::CharRef
            | TokenType::InlineStyle
            | TokenType::InlineMacroClose)
    }

    pub fn can_be_in_document_header(&self) -> bool {
        matches!(
            self.token_type(),
            TokenType::Heading1 | TokenType::Attribute
        ) || self.is_inline()
    }

    /// Performs some sanity-check validations; currently checking for characters that aren't
    /// allowed in, for example, IDs
    pub fn validate(&mut self) {
        match self.token_type() {
            TokenType::BlockAnchor | TokenType::CrossReference => {
                // no spaces or newlines inside
                if self.lexeme.contains(' ') || self.lexeme.contains('\n') {
                    self.token_type = TokenType::Text
                }
            }
            _ => {}
        }
    }
}

// would later add pub enum Section{Preface, Introduction, etc}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    // BLOCKS
    NewLineChar, // these are effectively semantic, so we should track them. Two in a row indicate
    // a blank line, which often signals the end of a block
    LineContinuation, // a "+" char at the end of a line; more or less creates a line break.

    // breaks
    ThematicBreak,
    PageBreak,

    Comment,

    // four char delimiters at new block
    PassthroughBlock, // i.e., "++++"
    SidebarBlock,     // i.e., "****"
    SourceBlock,      // i.e., "----"
    QuoteVerseBlock,  // i.e., "____"
    CommentBlock,     // i.e., "////"
    ExampleBlock,     // i.e., "====" (Also used for admonitions!)

    // two-char delimiters as new block
    OpenBlock, // i.e., --

    // two-char delimiters as new line
    OrderedListItem,
    UnorderedListItem,

    // block info markers
    BlockLabel, // ".Some text", specifically the r"^." here

    // headings
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,

    NotePara,      // NOTE:
    TipPara,       // TIP:
    ImportantPara, // IMPORTANT:
    CautionPara,   // CAUTION:
    WarningPara,   // WARNING:

    BlockContinuation, // a "+" all by itself on a line can signal continuation

    // INLINES
    // definition lists
    DescriptionListMarker, // just match "::" and the parser can figure it out

    // formatting tokens (inline markup)
    Strong,   // TK Handle bounded characters, e.g., **Some**thing -> <b>Some</b>thing
    Emphasis, // same applies above
    Monospace,
    Mark, // #text# or [.class]#text#

    Superscript, // ^super^
    Subscript,   // ~sub~

    // formatting tokens (inline markup)
    UnconstrainedStrong, // TK Handle bounded characters, e.g., **Some**thing -> <b>Some</b>thing
    UnconstrainedEmphasis, // same applies above
    UnconstrainedMonospace,
    UnconstrainedMark, // #text# or [.class]#text#

    // inline macros
    BlockImageMacro,
    InlineImageMacro,
    LinkMacro,
    FootnoteMacro, // requires a second pass? OR: do some kind of `self.last_token` check on the
    PassthroughInlineMacro,
    InlineMacroClose,

    // garden-variety text
    Text,

    // character reference, such as "&mdash;"
    CharRef,

    // document attributes
    Attribute,

    // attribute reference, e.g., "{my-thing}"
    AttributeReference,

    // End of File Token
    Eof,

    InlineStyle, // i.e., [.some_class], usually [.x]#applied here#

    // references, cross references TK
    // math blocks TK
    //Include,
    //StartTag, // tag::[]
    //EndTag,

    // Attributes, anchors and references
    BlockAnchor,
    ElementAttributes, // any of: [quote], [quote], [role="foo"], [#foo], etc.
    CrossReference,
}

impl TokenType {
    pub fn block_from_char(c: char) -> Self {
        match c {
            '+' => Self::PassthroughBlock,
            '*' => Self::SidebarBlock,
            '-' => Self::SourceBlock,
            '_' => Self::QuoteVerseBlock,
            '/' => Self::CommentBlock,
            '=' => Self::ExampleBlock,
            _ => panic!("Invalid character match to produce block TokenType"),
        }
    }

    pub fn clears_newline_after(&self) -> bool {
        matches!(
            self,
            TokenType::ElementAttributes
                | TokenType::SidebarBlock
                | TokenType::OpenBlock
                | TokenType::QuoteVerseBlock
                | TokenType::ExampleBlock
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Token, TokenType};
    use rstest::rstest;


    #[rstest]
    #[case::cross_references(TokenType::CrossReference)]
    #[case::block_anchor(TokenType::BlockAnchor)]
    fn invalid_space_invalidates_to_text(#[case] token_type: TokenType){
        let mut token = Token::new(token_type, " ".to_string(), None, 1, 1, 1);
        token.validate();
        assert_eq!(token.token_type(), TokenType::Text)
    }
}
