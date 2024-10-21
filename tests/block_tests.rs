use std::fs;

use asciidocr::{parser::Parser, scanner::Scanner};
use assert_json_diff::assert_json_eq;
use rstest::rstest;
use serde_json::{json, Value};

fn assert_parsed_doc_matches_expected_asg(adoc_fn: &str, asg_json_fn: &str) {
    let parsed_asg = json!(Parser::new().parse(Scanner::new(
        &fs::read_to_string(adoc_fn).expect(&format!("Unable to open {}", adoc_fn))
    )));

    let expected_asg: Value = serde_json::from_str(
        &fs::read_to_string(asg_json_fn).expect(&format!("Unable to open {}", asg_json_fn)),
    )
    .unwrap();

    assert_json_eq!(parsed_asg, expected_asg);
}

#[rstest]
#[case("tests/blocks/para-internal-line-break")]
#[case("tests/blocks/para-single-line")]
#[case("tests/blocks/para-two-paras-line-between")]
fn test_paragraphs(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[rstest]
#[case("tests/blocks/document-header")]
fn test_document(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}
