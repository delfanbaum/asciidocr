use std::{fs, path::PathBuf};

use asciidocr::{
    errors::ParserError,
    parser::Parser,
    scanner::{
        Scanner,
        tokens::{Token, TokenType},
    },
};
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
/// Test that we pull reference text from the target into reference as an inline but when it's an
/// include
fn test_reference_test_in_include() {
    let fn_pattern = "documents/references-include-fig";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Test that we pull reference text from the target into reference as an inline but when it's an
/// include
fn test_nested_includes() {
    let fn_pattern = "documents/nested-includes";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Test that we pull reference text from the target into reference as an inline but when it's an
/// include
fn test_title_only() {
    let fn_pattern = "documents/title-only";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Test dangling spans in title
fn test_title_dangling_span() {
    let fn_pattern = "documents/title-dangling-span";
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

/// The scanner does not produce email tokens, but if one ever arrives, the parser reports it
/// instead of panicking
#[test]
fn test_unexpected_token_is_an_error() {
    let tokens = vec![
        Ok(Token::new_default(
            TokenType::Email,
            "cats@dogs.foo".to_string(),
            None,
            1,
            1,
            13,
        )),
        Ok(Token::new_default(
            TokenType::Eof,
            String::new(),
            None,
            1,
            14,
            14,
        )),
    ];
    let result = Parser::new_no_target_resolution(PathBuf::new()).parse(tokens.into_iter());
    assert_eq!(
        result.err(),
        Some(ParserError::UnexpectedToken(1, "email address".to_string()))
    );
}
