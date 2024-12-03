use common::assert_rendered_html_matches_expected;

pub mod common;

#[test]
fn basic_format() {
    let fn_pattern = String::from("htmlbook/basic-format");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn many_lists() {
    let fn_pattern = String::from("htmlbook/many-lists");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn description_list() {
    let fn_pattern = String::from("htmlbook/description-list");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn roles_added_as_classes() {
    let fn_pattern = String::from("htmlbook/roles");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn quotes_with_attribution_and_citation() {
    let fn_pattern = String::from("htmlbook/quotes-attribution-citation");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn quotes_no_delimiter() {
    let fn_pattern = String::from("htmlbook/quotes-no-delimiter");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn line_continuation_creates_br_tags() {
    let fn_pattern = String::from("htmlbook/line-continuation");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn thematic_and_page_breaks() {
    let fn_pattern = String::from("htmlbook/breaks");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn footnotes() {
    let fn_pattern = String::from("htmlbook/footnote");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn passthroughs() {
    let fn_pattern = String::from("htmlbook/passthroughs");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn super_and_subscripts() {
    let fn_pattern = String::from("htmlbook/super-subscript");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn admonitions() {
    let fn_pattern = String::from("htmlbook/admonitions");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn delimited_verse() {
    let fn_pattern = String::from("htmlbook/verse");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn inline_images() {
    let fn_pattern = String::from("htmlbook/inline-image");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn block_images() {
    let fn_pattern = String::from("htmlbook/image-block-alt-text");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn block_figures() {
    let fn_pattern = String::from("htmlbook/image-block-figure");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
/// Note that eventually we'll want to change the ref text
fn cross_references() {
    let fn_pattern = String::from("htmlbook/cross-ref");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn block_anchors() {
    let fn_pattern = String::from("htmlbook/block-anchor");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn asides() {
    let fn_pattern = String::from("htmlbook/asides");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn table_simple() {
    let fn_pattern = String::from("htmlbook/table-simple");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn table_with_header() {
    let fn_pattern = String::from("htmlbook/table-header-implicit");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
fn callout_lists() {
    let fn_pattern = String::from("htmlbook/code-callouts");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}

#[test]
// it's just easier to reason about this in HTML than in JSON, so testing this here
fn section_nesting() {
    let fn_pattern = String::from("htmlbook/sections-nesting-and-close");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}
