pub mod common;

use std::env::current_dir;

use asciidocr::parser::{Parser, ParserError};
use asciidocr::scanner::Scanner;
use common::assert_parsed_doc_matches_expected_asg;

#[test]
fn test_offset_absolute() {
    let fn_pattern = "blocks/level-offset-absolute";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_offset_relative() {
    let fn_pattern = "blocks/level-offset-relative";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_offset_as_inline_attribute() {
    let fn_pattern = "blocks/level-offset-attributes";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_offest_too_big() {
    let adoc_str = String::from(":leveloffset: +7\n\n== Fail me\n\n");
    let result = Parser::new(current_dir().unwrap()).parse(Scanner::new(&adoc_str));
    assert!(result.is_err_and(|e| e == ParserError::HeadingOffsetError(3, 7)))
}

#[test]
fn test_offest_too_small() {
    let adoc_str = String::from(":leveloffset: -3\n\n== Fail me\n\n");
    let result = Parser::new(current_dir().unwrap()).parse(Scanner::new(&adoc_str));
    assert!(result.is_err_and(|e| e == ParserError::HeadingOffsetError(3, -3)))
}

#[test]
fn test_offest_level_0() {
    let adoc_str = String::from(":leveloffset: +1\n\n= Pass me\n\nOkay");
    let result = Parser::new(current_dir().unwrap()).parse(Scanner::new(&adoc_str));
    assert!(result.is_ok())
}
