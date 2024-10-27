pub mod common;
use std::fs;

use common::{assert_parsed_doc_matches_expected_asg, assert_parsed_doc_matches_expected_asg_from_str};
use rstest::rstest;

#[rstest]
#[case::para_single_line("blocks/para-single-line")]
#[case::para_two_lines_space_between("blocks/para-two-paras-line-between")]
#[case::para_internal_line_break("blocks/para-internal-line-break")]
fn test_paragraphs(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[rstest]
#[case::header_title_only("blocks/document-header")]
#[case::header_attrs("blocks/document-attr")]
fn test_document(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[rstest]
#[case::single_unordered("blocks/unordered-list")]
#[case::many_unordered("blocks/unordered-list-many-items")]
#[case::single_ordered("blocks/ordered-list")]
#[case::many_unordered("blocks/ordered-list-many-items")]
fn test_lists(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

// Note that "Heading1"s are treated as document or 0-levels
#[rstest]
#[case::heading2("== ", 1)]
#[case::heading3("=== ", 2)]
#[case::heading4("==== ", 3)]
#[case::heading5("===== ", 4)]
fn test_simple_heading_sections(#[case] heading_markup: &str, #[case] section_level: usize) {
    let offset = heading_markup.len(); // counts up to the beginning of the text
    let text_start = offset +1;
    let text_end = offset + "Section Title".len();
    
    let adoc_str = fs::read_to_string("tests/data/blocks/headings-sections.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("MARKUP", heading_markup);
    let asg_json_str = fs::read_to_string("tests/data/blocks/headings-sections.json")
        .expect("Unable to read asg json test template")
        .replace("91", &text_start.to_string()) // title text start
        .replace("92", &text_end.to_string()) // title text end
        .replace("93", &section_level.to_string()); // "level"
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}


#[rstest]
#[case::nested_two_in_one("blocks/nested-headings")]
#[case::nested_two_in_one_multiple("blocks/nested-headings-multiple")]
fn test_nexted_sections(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[rstest]
#[case::delimited_sidebar("blocks/delimited-block")]
fn test_delimited_blocks_no_meta(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}
