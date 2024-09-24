pub enum List {
    OrderedList(Box<Token>),
    UnorderedList(Box<Token>),
    DefinitionList(Box<Token>),
}

pub enum Token {
    // container blocks tokens: children, id, classes
    // There is probably a better classes data type to use
    Section(Box<Self>, String, Vec<String>), // note, handling "section styles" TK
    OpenBlock(Box<Self>, String, Vec<String>),
    Sidebar(Box<Self>, String, Vec<String>),
    Blockquote(Box<Self>, String, Vec<String>),
    Verse(Box<Self>, String, Vec<String>),
    PassthroughBlock(Vec<char>),

    // garden-variety blocks
    Heading(Box<Self>), // styles, ID on section
    Paragraph(Box<Self>, String, Vec<String>), // tokens, id, classes
    Source(Box<Self>), // or can this just be a Vec<char> since it's essentially a
    // pass-through?

    // lists
    List(Box<List>),
    ListItem(Box<Self>),
    DefListTerm(Box<Self>),
    DefListDesc(Box<Self>),

    // formatting tokens (inline markup)
    Bold(Vec<char>),
    Italic(Vec<char>),
    Monospace(Vec<char>),      // code
    Styled(Vec<char>, String), // i.e., [.some_class]#applied to a span#
    Superscript(Vec<char>),    // ^super^
    Subscript(Vec<char>),      // ~sub~

    // breaks
    ThematicBreak(Vec<char>), // do we need to keep the inner text? maybe not
    PageBreak(Vec<char>),     // do we need to keep the inner text? maybe not

    // links
    Link(Vec<char>, Box<Token>), // URL, link text

    // footnotes
    Footnote(Box<Self>),

    // includes
    Include(Vec<char>, String), // URI, tag
    StartTag(Vec<char>),
    EndTag(Vec<char>),

    // garden-variety text
    UnprocessedText(Vec<char>), // i.e., pre-inline checks
    Text(Vec<char>),

    // misc
    Comment(Vec<char>),
    PassthroughInline(Vec<char>),
    // references, cross references TK
    // math blocks TK

}
