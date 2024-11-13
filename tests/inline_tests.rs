pub mod common;
use std::fs;

use common::{
    assert_parsed_doc_matches_expected_asg, assert_parsed_doc_matches_expected_asg_from_str,
};
use rstest::rstest;

#[rstest]
#[case::emphasis("_", "emphasis")]
#[case::strong("*", "strong")]
#[case::code("`", "code")]
#[case::mark("#", "mark")]
fn test_spans_single_word(#[case] markup_char: &str, #[case] variant: &str) {
    let adoc_str = fs::read_to_string("tests/data/inlines/span-single-word.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("0", markup_char);
    let asg_json_str = fs::read_to_string("tests/data/inlines/span-single-word.json")
        .expect("Unable to read asg json test template")
        .replace("VARIANT", variant);
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[rstest]
#[case::emphasis("_")]
#[case::strong("*")]
#[case::code("`")]
#[case::mark("#")]
fn test_spans_dangling_at_front(#[case] markup_char: &str) {
    let adoc_str = String::from("0cat").replace("0", markup_char);
    let asg_json_str = fs::read_to_string("tests/data/inlines/span-dangling.json")
        .expect("Unable to read asg json test template")
        .replace("TEXT", &adoc_str);
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[rstest]
#[case::emphasis("_")]
#[case::strong("*")]
#[case::code("`")]
#[case::mark("#")]
fn test_spans_dangling_at_end(#[case] markup_char: &str) {
    let adoc_str = String::from("cat0").replace("0", markup_char);
    let asg_json_str = fs::read_to_string("tests/data/inlines/span-dangling.json")
        .expect("Unable to read asg json test template")
        .replace("TEXT", &adoc_str);
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[rstest]
#[case::emphasis("_", "emphasis")]
#[case::strong("*", "strong")]
#[case::code("`", "code")]
#[case::mark("#", "mark")]
fn test_spans_many_words(#[case] markup_char: &str, #[case] variant: &str) {
    let adoc_str = fs::read_to_string("tests/data/inlines/span-many-words.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("0", markup_char);
    let asg_json_str = fs::read_to_string("tests/data/inlines/span-many-words.json")
        .expect("Unable to read asg json test template")
        .replace("VARIANT", variant);
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[rstest]
#[case::emphasis("_", "emphasis")]
#[case::strong("*", "strong")]
#[case::code("`", "code")]
#[case::mark("#", "mark")]
fn test_spans_across_newlines(#[case] markup_char: &str, #[case] variant: &str) {
    let adoc_str = fs::read_to_string("tests/data/inlines/span-across-newline.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("0", markup_char);
    let asg_json_str = fs::read_to_string("tests/data/inlines/span-across-newline.json")
        .expect("Unable to read asg json test template")
        .replace("VARIANT", variant);
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[test]
fn test_spans_in_doc_title() {
    let fn_pattern = "inlines/document-header";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_links() {
    let fn_pattern = "inlines/links";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_footnotes() {
    let fn_pattern = "inlines/footnote";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Attribute refs are replaced as a part of a pre-processing step
fn test_attribute_ref_replacment() {
    let fn_pattern = "inlines/attribute-ref";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Attribute refs are replaced as a part of a pre-processing step
fn test_xref() {
    let fn_pattern = "inlines/cross-ref";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

/// Single words appear as such
#[test]
fn test_single_word() {
    let fn_pattern = "inlines/single-word";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

/// Inline classes can be applied
#[test]
fn test_inline_class() {
    let fn_pattern = "inlines/inline-class";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

/// Single words appear as such
#[test]
fn test_super_and_sub() {
    let fn_pattern = "inlines/super-subscript";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}


/// Inline images
#[test]
fn test_image_no_attributes() {
    let fn_pattern = "inlines/image";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}
