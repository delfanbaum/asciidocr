use common::{
    assert_rendered_htmlbook_embedded_matches_expected, assert_rendered_htmlbook_matches_expected,
    render_htmlbook_embedded_from_str,
};
use rstest::rstest;

pub mod common;

#[test]
fn basic_format() {
    let fn_pattern = String::from("htmlbook/basic-format");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn many_lists() {
    let fn_pattern = String::from("htmlbook/many-lists");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn description_list() {
    let fn_pattern = String::from("htmlbook/description-list");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn roles_added_as_classes() {
    let fn_pattern = String::from("htmlbook/roles");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn quotes_with_attribution_and_citation() {
    let fn_pattern = String::from("htmlbook/quotes-attribution-citation");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn quotes_no_delimiter() {
    let fn_pattern = String::from("htmlbook/quotes-no-delimiter");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn line_continuation_creates_br_tags() {
    let fn_pattern = String::from("htmlbook/line-continuation");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn thematic_and_page_breaks() {
    let fn_pattern = String::from("htmlbook/breaks");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn footnotes() {
    let fn_pattern = String::from("htmlbook/footnote");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn literal_blocks() {
    let fn_pattern = String::from("htmlbook/literal");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn literals_straight_quotes() {
    let fn_pattern = String::from("htmlbook/literals-straight-squo");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn passthroughs() {
    let fn_pattern = String::from("htmlbook/passthroughs");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn super_and_subscripts() {
    let fn_pattern = String::from("htmlbook/super-subscript");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn admonitions() {
    let fn_pattern = String::from("htmlbook/admonitions");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn delimited_verse() {
    let fn_pattern = String::from("htmlbook/verse");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn inline_images() {
    let fn_pattern = String::from("htmlbook/inline-image");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn block_images() {
    let fn_pattern = String::from("htmlbook/image-block-alt-text");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn block_figures() {
    let fn_pattern = String::from("htmlbook/image-block-figure");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
/// Note that eventually we'll want to change the ref text
fn cross_references() {
    let fn_pattern = String::from("htmlbook/cross-ref");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn block_anchors() {
    let fn_pattern = String::from("htmlbook/block-anchor");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn asides() {
    let fn_pattern = String::from("htmlbook/asides");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn table_simple() {
    let fn_pattern = String::from("htmlbook/table-simple");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn table_with_header() {
    let fn_pattern = String::from("htmlbook/table-header-implicit");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn callout_lists() {
    let fn_pattern = String::from("htmlbook/code-callouts");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
// it's just easier to reason about this in HTML than in JSON, so testing this here
fn section_nesting() {
    let fn_pattern = String::from("htmlbook/sections-nesting-and-close");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
// it's just easier to reason about this in HTML than in JSON, so testing this here
fn description_list_ordering() {
    let fn_pattern = String::from("htmlbook/description-list-ordering");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
// it's just easier to reason about this in HTML than in JSON, so testing this here
fn links() {
    let fn_pattern = String::from("htmlbook/links");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn embedded_has_no_document_wrapper() {
    let fn_pattern = String::from("htmlbook/embedded");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_embedded_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn document_header_sets_title() {
    let fn_pattern = String::from("htmlbook/document-header");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn block_titles() {
    let fn_pattern = String::from("htmlbook/block-titles");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn comments_are_not_rendered() {
    let fn_pattern = String::from("htmlbook/comments");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn leaf_block_titles_wrap_in_example() {
    let fn_pattern = String::from("htmlbook/leaf-block-titles");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn table_title_becomes_caption() {
    let fn_pattern = String::from("htmlbook/table-title");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn paragraph_title_is_dropped_but_content_kept() {
    let fn_pattern = String::from("htmlbook/paragraph-title-dropped");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn list_title_is_not_rendered_and_does_not_linger() {
    let fn_pattern = String::from("htmlbook/list-title-not-rendered");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn inline_admonition_takes_title() {
    let fn_pattern = String::from("htmlbook/admonition-inline-title");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_matches_expected(&adoc_fn, &html_fn)
}

/// Special characters in text and attribute values are escaped, while character references (from
/// replacements or written by the author) and passthrough blocks are left as they are
#[test]
fn special_characters_are_escaped() {
    let fn_pattern = String::from("htmlbook/escaping");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_htmlbook_embedded_matches_expected(&adoc_fn, &html_fn)
}

/// The other tests ignore whitespace, but it is significant inside `<pre>`: the newlines next to
/// the delimiters are dropped, while indentation and blank lines are kept
#[rstest]
#[case::listing(
    "----\n  indented\n\nlast\n----\n",
    "<pre data-type=\"programlisting\">  indented\n\nlast</pre>"
)]
#[case::literal("....\n  indented\n\nlast\n....\n", "<pre>  indented\n\nlast</pre>")]
#[case::verse(
    "[verse]\n____\n  indented\nlast\n____\n",
    "<pre data-type=\"verse\">  indented\nlast</pre>"
)]
#[case::verse_stanzas(
    "[verse]\n____\nOne\n\nTwo\n\n\nThree\n____\n",
    "<pre data-type=\"verse\">One\n\nTwo\n\n\nThree</pre>"
)]
fn test_preformatted_whitespace_is_exact(#[case] adoc: &str, #[case] expected: &str) {
    let html = render_htmlbook_embedded_from_str(adoc);
    assert!(html.contains(expected), "{html}");
}
