#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    token_type: TokenType,
    line: usize,
    //id: Option<String>,
    lexeme: String,
    literal: Option<String>, // the "literal value", e.e., an ITALIC's contents
                             //classes: Option<Vec<String>>,
}

impl Token {
    pub fn new(token_type: TokenType, line: usize, lexeme: String, literal: Option<String>) -> Self {
        Token {
            token_type,
            line,
            lexeme,
            literal,
        }
    }
}

// would later add pub enum Section{Preface, Introduction, etc}

#[derive(Debug, PartialEq, Eq)]
pub enum TokenType {
    BlankLine, // these are effectively semantic, so we should track them

    //Section -- inferred in parsing
    OpenBlock, // "--"

    // block info markers
    BlockLabel, // ".Some text", specifically the r"^." here

    Blockquote,            // [quote],
    BlockquoteAttribution, // quoted in [quote, quoted]
    BlockQuoteSource,      // source in [quote, quoted, source]

    Verse,            // [quote],
    VerseAttribution, // quoted in [quote, quoted]
    VerseSource,      // source in [quote, quoted, source]

    Source,         // [source]
    SourceLanguage, // language in [source,language]

    QuoteVerseBlock,  // i.e., "____"
    PassthroughBlock, // i.e., "++++"
    SourceBlock,      // i.e., "----"

    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,

    // lists -- we need the items, the parser will take care of the list part
    OrderedListItem,
    UnorderedListItem,
    DefinitionListItem,
    DefListTerm,
    DefListDesc,

    // formatting tokens (inline markup)
    Bold,   // TK Handle bounded characters, e.g., **Some**thing -> <b>Some</b>thing
    Italic, // same applies above
    Monospace,
    InlineStyle, // i.e., [.some_class], usually [.x]#applied here#
    Highlighted, // the part between the # above

    Superscript, // ^super^
    Subscript,   // ~sub~

    // breaks
    ThematicBreak,
    PageBreak,

    // links
    LinkUrl,
    LinkText,

    // footnotes
    Footnote, // requires a second pass? OR: do some kind of `self.last_token` check on the
    // scanner to determine if, for example, we've opened a link inside of our footnote

    // includes
    Include,
    StartTag, // tag::[]
    EndTag,

    // garden-variety text
    Text,

    // misc
    Comment,
    PassthroughInline,
    // references, cross references TK
    // math blocks TK
}
