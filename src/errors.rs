//! The crate's error types

#[cfg(feature = "docx")]
use docx_rs::DocxError;

#[cfg(feature = "docx")]
use crate::backends::docx::DocxRenderError;

/// Any of the crate's errors, for callers that don't need to tell them apart early
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum AsciidocrError {
    /// Scanning the source failed
    #[error(transparent)]
    Scanner(#[from] ScannerError),
    /// Parsing failed
    #[error(transparent)]
    Parser(#[from] ParserError),
    /// Building the graph failed
    #[error(transparent)]
    Asg(#[from] AsgError),
    /// Rendering the graph failed
    #[error(transparent)]
    Conversion(#[from] ConversionError),
    /// Reading or writing failed
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

/// Why the source couldn't be scanned into tokens
#[derive(thiserror::Error, PartialEq, Debug)]
#[non_exhaustive]
pub enum ScannerError {
    /// A token couldn't be made
    #[error(transparent)]
    Token(#[from] TokenError),
    /// An include tag line (`tag::name[]`) isn't written correctly; holds the line number and the
    /// text
    #[error("Line {0}: Invalid include tag pattern: {1}")]
    TagError(usize, String),
}

/// Why a document couldn't be parsed. Most problems in a document only log a warning; these are
/// the ones that stop the conversion
#[derive(thiserror::Error, PartialEq, Debug)]
#[non_exhaustive]
pub enum ParserError {
    /// The scanner failed
    #[error(transparent)]
    Scanner(#[from] ScannerError),
    /// A block couldn't be built
    #[error(transparent)]
    Block(#[from] BlockError),
    /// The graph couldn't be built
    #[error(transparent)]
    Asg(#[from] AsgError),
    /// A token the parser can't use yet (an email address); holds the line number and what it is
    #[error("Line {0}: Unexpected token: {1}")]
    UnexpectedToken(usize, String),
    /// Verbatim text ended without having started, on this line
    #[error("Line {0}: Invalid open_parse_after_as_text_type occurance")]
    OpenParse(usize),
    /// A delimited block was closed that wasn't open; holds the line number
    #[error("Line {0}: Attempted to close a non-existent delimited block")]
    DelimitedBlock(usize),
    /// The parser expected a delimited block, on this line, and found something else
    #[error("Line {0}: Unexpected block in Block::ParentBlock")]
    ParentBlock(usize),
    /// An included file (or, in some cases, another target) couldn't be found; holds the line
    /// number and the target
    #[error("Line {0}: Unable to resolve target: {1:?}")]
    TargetResolution(usize, String),
    /// The parser ran out of open blocks, on this line
    #[error("Line {0}: Tried to add last block when block stack was empty.")]
    BlockStack(usize),
    /// Something the parser didn't expect; holds the line number and a description
    #[error("Line {0}: error: {1}")]
    InternalError(usize, String),
    /// An attribute list couldn't be read; holds the line number and a description
    #[error("Line {0}: Attribute error: {1}")]
    AttributeError(usize, String),
}

/// Why a graph couldn't be rendered
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum ConversionError {
    /// The output file couldn't be created; holds its path and the reason
    #[cfg(feature = "docx")]
    #[error("Unable to create {0:?}: {1}")]
    OutputFile(std::path::PathBuf, #[source] std::io::Error),
    /// The DOCX library failed
    #[cfg(feature = "docx")]
    #[error(transparent)]
    Docx(#[from] DocxError),
    /// The graph couldn't be written as DOCX
    #[cfg(feature = "docx")]
    #[error(transparent)]
    DocxRender(#[from] DocxRenderError),
    /// An HTML template failed to render
    #[error(transparent)]
    MiniJinja(#[from] minijinja::Error),
}

/// Why a block couldn't be added to the graph
#[derive(thiserror::Error, PartialEq, Debug)]
#[non_exhaustive]
pub enum AsgError {
    /// A block couldn't be added
    #[error(transparent)]
    Block(#[from] BlockError),
}

/// Why blocks couldn't be put together. These point to a problem in the parser or an unusual
/// combination of blocks
#[derive(thiserror::Error, PartialEq, Debug)]
#[non_exhaustive]
pub enum BlockError {
    /// A list item had no list to go in
    #[error("Attempted to push dangling ListItem to parent block")]
    DanglingList,
    /// Something was added to a list item when no list was open
    #[error("Attempted to add to a list item when no list is open")]
    NoOpenList,
    /// Something other than a cell was added to a table
    #[error("Attempted to add something other than a TableCell to a Table")]
    TableCell,
    /// This kind of block can't hold what was added to it; holds the kind
    #[error("push_block not implemented for {0}")]
    NotImplemented(String),
    /// An operation for one kind of block was used on another
    #[error("Incorrect function call: consolidate_table_info on non-table block")]
    IncorrectCall,
    /// A block has no location
    #[error("Missing location information for block")]
    Location,
    /// A block was made from a token that doesn't start one
    #[error("Tried to create a block from an invalid Token")]
    InvalidToken,
    /// This kind of block can't have attributes; holds the kind
    #[error("Blocks of type {0} do not accept metadata")]
    InvalidMetadata(String),
}

/// Why a token couldn't be made
#[derive(thiserror::Error, PartialEq, Debug)]
#[non_exhaustive]
pub enum TokenError {
    /// A character was taken for a block delimiter that isn't one
    #[error("Invalid character match to produce block TokenType")]
    CharacterMatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_error_shows_line_and_description() {
        let error = ParserError::AttributeError(5, "foo does not contain a named attribute".into());
        assert_eq!(
            error.to_string(),
            "Line 5: Attribute error: foo does not contain a named attribute"
        );
    }
}
