use std::{fs, path::PathBuf};

use asciidocr::{parser::Parser, scanner::Scanner};
use common::assert_parsed_doc_matches_expected_asg;

pub mod common;

#[test]
/// Test that we pull reference text from the target into reference as an inline
fn test_reference_test_insertion() {
    let fn_pattern = "documents/references-titles";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
///Smoke test for "have we covered enough" -- will be added to as we go along and do not panic
fn test_targeted_coverage() {
    let adoc_fn = "tests/data/documents/minimum.adoc";
    let _ = Parser::new(PathBuf::from(adoc_fn)).parse(Scanner::new(
        &fs::read_to_string(adoc_fn).expect("Unable to find adoc"),
    ));
}
