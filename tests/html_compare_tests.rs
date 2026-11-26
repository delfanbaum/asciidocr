//! Tests of the whitespace-aware HTML comparison in `common::html`, which the HTML tests use
pub mod common;

use common::html::{assert_html_eq, canonical_html, select};
use rstest::rstest;

/// HTML that renders the same compares equal
#[rstest]
#[case::indentation_between_blocks("<div>\n  <p>a</p>\n</div>\n", "<div><p>a</p></div>")]
#[case::runs_of_whitespace("<p>a\n   b</p>", "<p>a b</p>")]
#[case::whitespace_at_block_edges("<p>\n  a\n</p>", "<p>a</p>")]
#[case::whitespace_after_br("<p>a<br>\nb</p>", "<p>a<br>b</p>")]
#[case::attribute_order("<p id=\"x\" class=\"a\">t</p>", "<p class=\"a\" id=\"x\">t</p>")]
#[case::comments("<p>a<!-- note --></p>", "<p>a</p>")]
// the HTML parser drops a newline directly after `<pre>`, as browsers do
#[case::newline_after_pre("<pre>\na</pre>", "<pre>a</pre>")]
#[case::document_indentation(
    "<!DOCTYPE html>\n<html>\n<head>\n<title>T</title>\n</head>\n<body>\n<p>a</p>\n</body>\n</html>\n",
    "<!DOCTYPE html><html><head><title>T</title></head><body><p>a</p></body></html>"
)]
fn test_same_rendering(#[case] actual: &str, #[case] expected: &str) {
    assert_html_eq(actual, expected);
}

/// HTML that renders differently doesn't
#[rstest]
#[case::spaces_next_to_inline_elements("<p>a <b>b</b> c</p>", "<p>a<b>b</b>c</p>")]
#[case::stray_newline_in_link("<p><a>x</a><a>\ny</a></p>", "<p><a>x</a><a>y</a></p>")]
#[case::whitespace_in_pre("<pre>a\n  b</pre>", "<pre>a\n b</pre>")]
#[case::class_order("<p class=\"a b\">t</p>", "<p class=\"b a\">t</p>")]
#[case::words("<p>ab</p>", "<p>a b</p>")]
#[case::no_break_space("<p>a&nbsp;b</p>", "<p>a b</p>")]
fn test_different_rendering(#[case] actual: &str, #[case] expected: &str) {
    assert_ne!(canonical_html(actual), canonical_html(expected));
}

#[test]
#[should_panic]
fn test_assert_html_eq_fails_on_a_difference() {
    assert_html_eq("<p>a b</p>", "<p>ab</p>");
}

/// The canonical form shows text quoted, so whitespace is visible
#[test]
fn test_canonical_form() {
    assert_eq!(
        canonical_html("<div class=\"x\">\n<p>a <em>b</em></p>\n<pre>c\n d</pre></div>"),
        "<div class=\"x\">\n  <p>\n    \"a \"\n    <em>\n      \"b\"\n  <pre>\n    \"c\\n d\"\n"
    );
}

/// `select` gives the HTML of the matching elements, to compare part of a page
#[test]
fn test_select() {
    let html = "<div><dl><dt>a</dt><dd><p>b</p></dd></dl><p>x</p><dl><dt>c</dt></dl></div>";
    assert_html_eq(
        &select(html, "dl"),
        "<dl><dt>a</dt><dd><p>b</p></dd></dl><dl><dt>c</dt></dl>",
    );
    assert_html_eq(&select(html, "dd p"), "<p>b</p>");
    assert_eq!(select(html, "table"), "");
}
