pub mod common;
use std::fs;

use common::{
    assert_parsed_doc_matches_expected_asg, assert_parsed_doc_matches_expected_asg_from_str,
};
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
#[case::complex_unordered("blocks/unordered-list-complex")]
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
    let text_start = offset + 1;
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
#[case::delimited_sidebar("****", "sidebar")]
#[case::delimited_sidebar("====", "example")]
#[case::delimited_sidebar("____", "quote")]
#[case::delimited_sidebar("--", "open")]
fn test_delimited_blocks_no_meta(#[case] delimiter: &str, #[case] name: &str) {
    let adoc_str = fs::read_to_string("tests/data/blocks/delimited-block.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("****", delimiter);
    let mut asg_json_str = fs::read_to_string("tests/data/blocks/delimited-block.json")
        .expect("Unable to read asg json test template")
        .replace("sidebar", name)
        .replace("****", delimiter);

    // fix location numbering
    if name == "open" {
        asg_json_str = asg_json_str.replace("5, \"col\": 4", "5, \"col\": 2");
    }

    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[rstest]
#[case::note("NOTE")]
#[case::note("TIP")]
#[case::note("IMPORTANT")]
#[case::note("CAUTION")]
#[case::note("WARNING")]
fn test_admontions_non_delimited(#[case] admonition: &str) {
    let text_start = admonition.len() + 3;
    let text_end = text_start + 9; // "Notice me!"
    let adoc_str = fs::read_to_string("tests/data/blocks/admonition-inline.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("NOTE", admonition);
    let asg_json_str = fs::read_to_string("tests/data/blocks/admonition-inline.json")
        .expect("Unable to read asg json test template")
        .replace("TEXT_START", &text_start.to_string())
        .replace("TEXT_END", &text_end.to_string())
        .replace("DELIMITER", &format!("{}: ", admonition))
        .replace("ADMONITION", &admonition.to_lowercase());

    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}
