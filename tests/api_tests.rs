//! The library as another crate sees it
pub mod common;
use std::str::FromStr;

use asciidocr::{graph::asg::Asg, parser::Parser, scanner::Scanner};
use serde_json::json;

/// Without a file, includes are found from the current directory (they were looked for in its
/// parent)
#[test]
fn test_includes_resolve_from_the_current_directory() {
    // tests run in the crate's directory
    let adoc = "include::tests/data/blocks/include-offset-part.adoc[]\n";
    let from_parser = Parser::default()
        .parse(Scanner::new(adoc))
        .expect("Unable to parse");
    let from_str = Asg::from_str(adoc).expect("Unable to parse");
    for graph in [from_parser, from_str] {
        assert_eq!(graph.blocks.len(), 1, "{}", json!(graph));
    }
}

/// A parser can be used again, and nothing of one document carries into the next
#[test]
fn test_parser_reuse() {
    let first = "= One\nJane Doe\n:name: value\n:leveloffset: 2\n\nA:: b\n\n[[id]]\n.Title\n====\nopen\n\n* item\n+\n";
    let second = "{name}\n\n== Two\n\ntext\nmore\n";
    let mut parser = Parser::default();
    parser.resolve_targets = false;
    parser.parse(Scanner::new(first)).expect("Unable to parse");
    let reused = parser.parse(Scanner::new(second)).expect("Unable to parse");
    assert!(!parser.resolve_targets, "the setting is kept");

    let fresh = Parser::default()
        .parse(Scanner::new(second))
        .expect("Unable to parse");
    assert_eq!(json!(reused), json!(fresh));
}

/// An output file that can't be created is an error
#[cfg(feature = "docx")]
#[test]
fn test_docx_output_that_cannot_be_created() {
    use asciidocr::backends::docx::render_docx;

    let graph = Asg::from_str("text\n").expect("Unable to parse");
    let directory = tempfile::tempdir().expect("Unable to make a directory");
    let result = render_docx(&graph, &directory.path().join("missing/out.docx"));
    assert!(result.is_err());
}
