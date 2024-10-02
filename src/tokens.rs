#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub token_type: TokenType,
    //id: Option<String>,
    pub lexeme: String,          // raw string of code
    pub literal: Option<String>, // our literals are only ever strings (or represented as such)
    pub line: usize,
}

impl Default for Token {
    fn default() -> Self {
        // defaults to EOF
        Token {
            token_type: TokenType::Eof,
            lexeme: "\0".to_string(),
            literal: None,
            line: 0,
        }
    }
}

impl Token {
    pub fn new(
        token_type: TokenType,
        lexeme: String,
        literal: Option<String>,
        line: usize,
    ) -> Self {
        Token {
            token_type,
            lexeme,
            literal,
            line,
        }
    }

    pub fn token_type(&self) -> TokenType {
        self.token_type
    }
}

// would later add pub enum Section{Preface, Introduction, etc}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    // BLOCKS
    UnprocessedLine, // placeholder token for reprocessing
    NewLineChar, // these are effectively semantic, so we should track them. Two in a row indicate
    // a blank line, which often signals the end of a block

    // breaks
    ThematicBreak,
    PageBreak,

    Comment,

    // four char delimiters at new block
    PassthroughBlock, // i.e., "++++"
    AsideBlock,       // i.e., "****"
    SourceBlock,      // i.e., "----"
    QuoteVerseBlock,  // i.e., "____"
    CommentBlock,  // i.e., "////"

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

    Blockquote,            // [quote],
    //BlockquoteAttribution, // quoted in [quote, quoted]
    //BlockQuoteSource,      // source in [quote, quoted, source]

    Verse,            // [quote],
    //VerseAttribution, // quoted in [quote, quoted]
    //VerseSource,      // source in [quote, quoted, source]
    
    BlockContinuation, // a "+" all by itself on a line can signal continuation

    Source,         // [source]
    //SourceLanguage, // language in [source,language]
    //
    //// includes
    //Include,
    //StartTag, // tag::[]
    //EndTag,

    // INLINES

    // definition lists
    DefListTerm, // starts with new line or block?
    DefListDesc,

    // formatting tokens (inline markup)
    Bold,   // TK Handle bounded characters, e.g., **Some**thing -> <b>Some</b>thing
    Italic, // same applies above
    Monospace,
    InlineStyle, // i.e., [.some_class], usually [.x]#applied here#
    Highlighted, // the part between the # above

    Superscript, // ^super^
    Subscript,   // ~sub~

    // links
    LinkUrl,
    LinkText,

    // footnotes
    Footnote, // requires a second pass? OR: do some kind of `self.last_token` check on the
    // scanner to determine if, for example, we've opened a link inside of our footnote


    // garden-variety text
    Text,

    // misc
    PassthroughInline,
    // references, cross references TK
    // math blocks TK

    // End of File Token
    Eof,
}

impl TokenType {
    pub fn block_from_char(c: char) -> Self {
        match c {
            '+' => Self::PassthroughBlock,
            '*' => Self::AsideBlock,
            '-' => Self::SourceBlock,
            '_' => Self::QuoteVerseBlock,
            '/' => Self::CommentBlock,
            _ => panic!("Invalid character match to produce block TokenType"),
        }
    }
}
