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
    #[error("Line {0}: Invalid headling level")]
    HeadingLevelError(usize),
    #[error("Line {0}: Invalid include tag pattern: {1}")]
    TagError(usize, String),
    #[error("Line {0}: Unknown scanner error")]
    UnknownError(usize),
}

#[derive(thiserror::Error, PartialEq, Debug)]
pub enum ParserError {
    #[error(transparent)]
    Scanner(#[from] ScannerError),
    #[error(transparent)]
    Block(#[from] BlockError),
    #[error(transparent)]
    Asg(#[from] AsgError),
    #[error("Line {0}: Unexpected token: {1}")]
    UnexpectedToken(usize, String),
    #[error("Line {0}: Invalid open_parse_after_as_text_type occurance")]
    OpenParse(usize),
    #[error("Line {0}: Attempted to close a non-existent delimited block")]
    DelimitedBlock(usize),
    #[error("Line {0}: Unexpected block in Block::ParentBlock")]
    ParentBlock(usize),
    #[error("Line {0}: Unable to resolve target: {1:?}")]
    TargetResolution(usize, String),
    #[error("Line {0}: Tried to add last block when block stack was empty.")]
    BlockStack(usize),
    #[error("Line {0}: error: {1}")]
    InternalError(usize, String),
    #[error("Line {0}: Attribute error: {1}")]
    AttributeError(usize, String),
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
    MiniJinja(#[from] minijinja::Error),
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
    #[error("Attempted to add to a list item when no list is open")]
    NoOpenList,
    #[error("Attempted to add something other than a TableCell to a Table")]
    TableCell,
    #[error("push_block not implemented for {0}")]
    NotImplemented(String),
    #[error("Incorrect function call: consolidate_table_info on non-table block")]
    IncorrectCall,
    #[error("Missing location information for block")]
    Location,
    #[error("Tried to create a block from an invalid Token")]
    InvalidToken,
    #[error("Blocks of type {0} do not accept metadata")]
    InvalidMetadata(String),
}

#[derive(thiserror::Error, PartialEq, Debug)]
pub enum TokenError {
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
