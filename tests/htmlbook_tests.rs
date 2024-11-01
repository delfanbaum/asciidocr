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
fn many_lines() {
    let fn_pattern = String::from("htmlbook/many-lists");
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_html_matches_expected(&adoc_fn, &html_fn)
}
