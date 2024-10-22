mod common;
use common::assert_parsed_doc_matches_expected_asg;
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

