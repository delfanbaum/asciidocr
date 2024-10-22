use std::fs;

use asciidocr::{parser::Parser, scanner::Scanner};
use assert_json_diff::assert_json_eq;
use serde_json::{json, Value};

pub fn assert_parsed_doc_matches_expected_asg(adoc_fn: &str, asg_json_fn: &str) {
    let test_dir = "tests/data/";
    let parsed_asg = json!(Parser::new().parse(Scanner::new(
        &fs::read_to_string(&format!("{}{}", test_dir, adoc_fn)).expect("Unable to find adoc")
    )));

    let expected_asg: Value = serde_json::from_str(
        &fs::read_to_string(&format!("{}{}", test_dir, asg_json_fn))
            .expect("Unable to find asg json"),
    )
    .unwrap();

    assert_json_eq!(parsed_asg, expected_asg);
}
