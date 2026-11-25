pub mod common;

use common::{
    assert_rendered_asciidoctor_embedded_matches_expected,
    assert_rendered_asciidoctor_html_matches_expected, render_asciidoctor_embedded_from_str,
};
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

/// Embedded output (document contents only); the expected html is what `asciidoctor -s` produces
#[rstest]
#[case::inlines("asciidoctor-html/embedded/inlines")]
#[case::leaf_blocks("asciidoctor-html/embedded/leaf-blocks")]
#[case::breaks("asciidoctor-html/embedded/breaks")]
#[case::parent_blocks("asciidoctor-html/embedded/parent-blocks")]
#[case::description_list("asciidoctor-html/embedded/description-list")]
#[case::table("asciidoctor-html/embedded/table")]
#[case::xrefs("asciidoctor-html/embedded/xrefs")]
fn test_embedded(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let html_fn = format!("{}.html", fn_pattern);
    assert_rendered_asciidoctor_embedded_matches_expected(&adoc_fn, &html_fn)
}

/// The other tests ignore whitespace, but it is significant inside `<pre>`: the newlines next to
/// the delimiters are dropped, while indentation and blank lines are kept (as Asciidoctor does)
#[rstest]
#[case::listing("----\n  indented\n\nlast\n----\n", "<pre>  indented\n\nlast</pre>")]
#[case::literal("....\n  indented\n\nlast\n....\n", "<pre>  indented\n\nlast</pre>")]
#[case::verse(
    "[verse]\n____\n  indented\nlast\n____\n",
    "<pre class=\"content\">  indented\nlast</pre>"
)]
fn test_preformatted_whitespace_is_exact(#[case] adoc: &str, #[case] expected: &str) {
    let html = render_asciidoctor_embedded_from_str(adoc);
    assert!(html.contains(expected), "{html}");
}
