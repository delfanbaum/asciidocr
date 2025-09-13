#[cfg(feature = "docx")]
use docx_rs::DocxError;

#[cfg(feature = "docx")]
use crate::backends::docx::document::DocxRenderError;

#[derive(thiserror::Error, Debug)]
pub enum AsciidocrError {
    #[error(transparent)]
    Scanner(#[from] ScannerError),
    #[error(transparent)]
    Parser(#[from] ParserError),
    #[error(transparent)]
    Asg(#[from] AsgError),
    #[error(transparent)]
    Conversion(#[from] ConversionError),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

#[derive(thiserror::Error, PartialEq, Debug)]
pub enum ScannerError {
    #[error(transparent)]
    Token(#[from] TokenError),
    #[error("Invalid headling level at line {0}")]
    HeadingLevelError(usize),
    #[error("Invalid include tag pattern: {0}")]
    TagError(String),
}

#[derive(thiserror::Error, PartialEq, Debug)]
pub enum ParserError {
    #[error(transparent)]
    Scanner(#[from] ScannerError),
    #[error(transparent)]
    Block(#[from] BlockError),
    #[error(transparent)]
    Asg(#[from] AsgError),
    #[error("Parse error line {0}: Level 0 headings are only allowed at the top of a document")]
    TopLevelHeading(usize),
    #[error("Parse error line {0}: Invalid open_parse_after_as_text_type occurance")]
    OpenParse(usize),
    #[error("Parse error: Attempted to close a non-existent delimited block")]
    DelimitedBlock,
    #[error("Parse error line {0}: Unexpected block in Block::ParentBlock")]
    ParentBlock(usize),
    #[error(
        "Parse error line {0}: Invalid heading level; parser level offest at the time of error was: {1}"
    )]
    HeadingOffsetError(usize, i8),
    #[error("Parse error line {0}: Unable to resolve target: {1:?}")]
    TargetResolution(usize, String),
    #[error("Parser error: Tried to add last block when block stack was empty.")]
    BlockStack,
    #[error("Parse error: invalid block continuation; no previous block")]
    BlockContinuation,
    #[error("Parse error: {0}")]
    InternalError(String),
    #[error("Attribute error: {0}")]
    AttributeError(String),
}

#[derive(thiserror::Error, Debug)]
pub enum ConversionError {
    #[cfg(feature = "docx")]
    #[error(transparent)]
    Docx(#[from] DocxError),
    #[cfg(feature = "docx")]
    #[error(transparent)]
    DocxRender(#[from] DocxRenderError),
    #[error(transparent)]
    TeraError(#[from] tera::Error),
}

#[derive(thiserror::Error, PartialEq, Debug)]
pub enum AsgError {
    #[error(transparent)]
    Block(#[from] BlockError),
}

#[derive(thiserror::Error, PartialEq, Debug)]
pub enum BlockError {
    #[error("Attempted to push dangling ListItem to parent block")]
    DanglingList,
    #[error("Attempted to add something other than a TableCell to a Table")]
    TableCell,
    #[error("push_block not implemented for {0}")]
    NotImplemented(String),
    #[error("Incorrect function call: consolidate_table_info on non-table block")]
    IncorrectCall,
    #[error("Missing location information for block")]
    Location,
    #[error("Tried to create a block from an invalid Token.")]
    InvalidToken,
    #[error("Footnote error: {0}")]
    Footnote(String),
}

#[derive(thiserror::Error, PartialEq, Debug)]
pub enum TokenError {
    #[error("Invalid character match to produce block TokenType")]
    CharacterMatch,
}
