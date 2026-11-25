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
#[case::images("asciidoctor-html/embedded/images")]
#[case::titles("asciidoctor-html/embedded/titles")]
#[case::ids_and_roles("asciidoctor-html/embedded/ids-and-roles")]
#[case::footnotes("asciidoctor-html/embedded/footnotes")]
#[case::verse_attribution("asciidoctor-html/embedded/verse-attribution")]
#[case::preamble("asciidoctor-html/embedded/preamble")]
#[case::no_preamble("asciidoctor-html/embedded/no-preamble")]
#[case::escaping("asciidoctor-html/embedded/escaping")]
#[case::section_ids("asciidoctor-html/embedded/section-ids")]
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
#[case::verse_stanzas(
    "[verse]\n____\nOne\n\nTwo\n\n\nThree\n____\n",
    "<pre class=\"content\">One\n\nTwo\n\n\nThree</pre>"
)]
fn test_preformatted_whitespace_is_exact(#[case] adoc: &str, #[case] expected: &str) {
    let html = render_asciidoctor_embedded_from_str(adoc);
    assert!(html.contains(expected), "{html}");
}

/// A document with a little of everything renders without template errors, in both modes
#[test]
fn test_minimum_document_renders() {
    let adoc = std::fs::read_to_string("tests/data/documents/minimum.adoc").unwrap();
    let html = render_asciidoctor_embedded_from_str(&adoc);
    assert!(!html.trim().is_empty());
}

/// Asciidoctor itself outputs quote attributions as written; we escape them, since they can hold
/// any characters (so this is not part of the fixture compared against Asciidoctor)
#[test]
fn test_attribution_is_escaped() {
    let html = render_asciidoctor_embedded_from_str(
        "[quote, Smith & Sons, A <Book>]\n____\nA quote.\n____\n\n[verse, Jo & Co]\n____\nA poem.\n____\n",
    );
    assert!(html.contains("&#8212; Smith &amp; Sons<br>"), "{html}");
    assert!(html.contains("<cite>A &lt;Book&gt;</cite>"), "{html}");
    assert!(html.contains("&#8212; Jo &amp; Co"), "{html}");
}

/// Passthroughs are output as written, and the text around them is still escaped
#[test]
fn test_inline_passthrough_is_not_escaped() {
    let html = render_asciidoctor_embedded_from_str("a < b pass:[<b>raw</b>] & c\n");
    assert!(
        html.contains("<p>a &lt; b <b>raw</b> &amp; c</p>"),
        "{html}"
    );
}

/// Text after a bare URL follows the link rather than becoming its text, and keeps its own
/// escaping (or lack of it, for passthroughs); a link macro still takes the text in its brackets
#[rstest]
#[case::text_after(
    "See http://example.com then.\n",
    "<p>See <a href=\"http://example.com\" class=\"bare\">http://example.com</a> then.</p>"
)]
#[case::next_line(
    "A http://example.com/bar/\nnext line.\n",
    "<p>A <a href=\"http://example.com/bar/\" class=\"bare\">http://example.com/bar/</a>\nnext line.</p>"
)]
#[case::passthrough_after(
    "Raw http://example.com pass:[<b>y</b>]\n",
    "<a href=\"http://example.com\" class=\"bare\">http://example.com</a> <b>y</b></p>"
)]
#[case::link_macro(
    "Macro http://example.com[*text*] after.\n",
    "<p>Macro <a href=\"http://example.com\"><strong>text</strong></a> after.</p>"
)]
fn test_text_after_bare_url(#[case] adoc: &str, #[case] expected: &str) {
    let html = render_asciidoctor_embedded_from_str(adoc);
    assert!(html.contains(expected), "{html}");
}
