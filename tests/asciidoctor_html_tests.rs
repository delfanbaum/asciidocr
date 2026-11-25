pub mod common;

use common::assert_rendered_asciidoctor_html_matches_expected;
use rstest::rstest;

/// Full documents: the document shell (header, content, footer) around Asciidoctor-styled blocks
#[rstest]
#[case::paragraph("asciidoctor-html/paragraph")]
#[case::unordered_list("asciidoctor-html/blocks-unordered-list")]
#[case::ordered_list("asciidoctor-html/blocks-ordered-list")]
#[case::code("asciidoctor-html/blocks-code")]
#[case::open("asciidoctor-html/blocks-open")]
fn test_full_documents(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_asciidoctor_html_matches_expected(&adoc_fn, &html_fn)
}
