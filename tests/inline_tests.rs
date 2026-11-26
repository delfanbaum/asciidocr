pub mod common;
use std::fs;

use common::{
    assert_parsed_doc_matches_expected_asg, assert_parsed_doc_matches_expected_asg_from_str,
    parsed_text, render_asciidoctor_embedded_from_str,
};
use rstest::rstest;

#[rstest]
#[case::emphasis("_", "emphasis")]
#[case::strong("*", "strong")]
#[case::code("`", "code")]
#[case::mark("#", "mark")]
fn test_spans_single_word(#[case] markup_char: &str, #[case] variant: &str) {
    let adoc_str = fs::read_to_string("tests/data/inlines/span-single-word.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("0", markup_char);
    let asg_json_str = fs::read_to_string("tests/data/inlines/span-single-word.json")
        .expect("Unable to read asg json test template")
        .replace("VARIANT", variant);
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[rstest]
#[case::emphasis("_", "emphasis")]
#[case::strong("*", "strong")]
#[case::code("`", "code")]
#[case::mark("#", "mark")]
fn test_spans_with_chars_between(#[case] markup_char: &str, #[case] variant: &str) {
    let adoc_str = fs::read_to_string("tests/data/inlines/span-chars-between.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("0", markup_char);
    let mut asg_json_str = fs::read_to_string("tests/data/inlines/span-chars-between.json")
        .expect("Unable to read asg json test template")
        .replace("VARIANT", variant);
    // an apostrophe in code stays as written (see the `literals-straight-squo` fixture)
    if variant == "code" {
        asg_json_str = asg_json_str.replace("&#8217;", "'");
    }
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[test]
fn test_spans_inside_spans() {
    let adoc_str = fs::read_to_string("tests/data/inlines/span-inside-spans.adoc")
        .expect("Unable to read asciidoc test template");
    let asg_json_str = fs::read_to_string("tests/data/inlines/span-inside-spans.json")
        .expect("Unable to read asg json test template");
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[test]
fn test_spans_inside_spans_mixed() {
    let adoc_str = fs::read_to_string("tests/data/inlines/span-inside-spans-mixed.adoc")
        .expect("Unable to read asciidoc test template");
    let asg_json_str = fs::read_to_string("tests/data/inlines/span-inside-spans-mixed.json")
        .expect("Unable to read asg json test template");
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[rstest]
#[case::emphasis("_")]
#[case::strong("*")]
#[case::code("`")]
#[case::mark("#")]
fn test_spans_dangling_at_front(#[case] markup_char: &str) {
    let adoc_str = String::from("0cat").replace("0", markup_char);
    let asg_json_str = fs::read_to_string("tests/data/inlines/span-dangling.json")
        .expect("Unable to read asg json test template")
        .replace("TEXT", &adoc_str);
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[rstest]
#[case::emphasis("_")]
#[case::strong("*")]
#[case::code("`")]
#[case::mark("#")]
fn test_spans_dangling_at_end(#[case] markup_char: &str) {
    let adoc_str = String::from("cat0").replace("0", markup_char);
    let asg_json_str = fs::read_to_string("tests/data/inlines/span-dangling.json")
        .expect("Unable to read asg json test template")
        .replace("TEXT", &adoc_str);
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[rstest]
#[case::emphasis("_", "emphasis")]
#[case::strong("*", "strong")]
#[case::code("`", "code")]
#[case::mark("#", "mark")]
fn test_spans_many_words(#[case] markup_char: &str, #[case] variant: &str) {
    let adoc_str = fs::read_to_string("tests/data/inlines/span-many-words.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("0", markup_char);
    let asg_json_str = fs::read_to_string("tests/data/inlines/span-many-words.json")
        .expect("Unable to read asg json test template")
        .replace("VARIANT", variant);
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[rstest]
#[case::emphasis("_", "emphasis")]
#[case::strong("*", "strong")]
#[case::code("`", "code")]
#[case::mark("#", "mark")]
fn test_spans_across_newlines(#[case] markup_char: &str, #[case] variant: &str) {
    let adoc_str = fs::read_to_string("tests/data/inlines/span-across-newline.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("0", markup_char);
    let asg_json_str = fs::read_to_string("tests/data/inlines/span-across-newline.json")
        .expect("Unable to read asg json test template")
        .replace("VARIANT", variant);
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

/// A span that closes at the end of a line must not swallow a span that opens at the start of
/// the next line
#[rstest]
#[case::emphasis("_", "emphasis")]
#[case::strong("*", "strong")]
#[case::code("`", "code")]
#[case::mark("#", "mark")]
fn test_spans_adjacent_across_newline(#[case] markup_char: &str, #[case] variant: &str) {
    let adoc_str = fs::read_to_string("tests/data/inlines/spans-adjacent-across-newline.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("0", markup_char);
    let asg_json_str = fs::read_to_string("tests/data/inlines/spans-adjacent-across-newline.json")
        .expect("Unable to read asg json test template")
        .replace("VARIANT", variant);
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

/// Like Asciidoctor, a newline that precedes a span nested in an open span is text in the
/// enclosing span, not the start of the nested span
#[rstest]
#[case::after_closed_span("inlines/nested-span-after-newline-closed-span")]
#[case::after_text("inlines/nested-span-after-newline-text")]
fn test_nested_span_after_newline(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_spans_in_doc_title() {
    let fn_pattern = "inlines/document-header";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_links() {
    let fn_pattern = "inlines/links";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_footnotes() {
    let fn_pattern = "inlines/footnote";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Attribute refs are replaced as a part of a pre-processing step
fn test_attribute_ref_replacment() {
    let fn_pattern = "inlines/attribute-ref";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Attribute refs are replaced as a part of a pre-processing step
fn test_xref() {
    let fn_pattern = "inlines/cross-ref";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

/// Single words appear as such
#[test]
fn test_single_word() {
    let fn_pattern = "inlines/single-word";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

/// Inline classes can be applied
#[test]
fn test_inline_class() {
    let fn_pattern = "inlines/inline-class";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

/// Single words appear as such
#[test]
fn test_super_and_sub() {
    let fn_pattern = "inlines/super-subscript";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

/// charref entities are processed by themselves
#[test]
fn test_charref() {
    let fn_pattern = "inlines/charrefs";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

/// Inline images
#[test]
fn test_image_no_attributes() {
    let fn_pattern = "inlines/image";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_typographer_quotes() {
    let fn_pattern = "inlines/quote_replacements";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_inline_markup_edge_cases() {
    let fn_pattern = "inlines/inline_markup_edge_cases";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

/// Markup that is never closed (or never finished) is just text; it must not stop the parser
#[rstest]
#[case::lone_markers("*_\n", "*_")]
#[case::unclosed_span_before_span("A *_x_ and unclosed\n", "A *x and unclosed")]
#[case::unclosed_span_inside_span("A *b _c\n", "A *b _c")]
#[case::unclosed_xref("see <<oops\n", "see <<oops")]
#[case::unclosed_xref_at_end_of_file("see <<oops", "see <<oops")]
#[case::xref_with_one_bracket("see <<x>\n", "see <<x>")]
#[case::unclosed_anchor("[[abc\npara\n", "[[abc\npara")]
fn test_unfinished_markup_is_text(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(parsed_text(adoc), expected);
}

/// With the `icons` attribute, the comment markup ahead of a callout (`// <1>`) is dropped, as in
/// Asciidoctor, and the code and the callout's note are kept
#[test]
fn test_code_callouts_with_icons_attribute() {
    let text = parsed_text(":icons: true\n\n[source,c]\n----\nint x; // <1>\n----\n<1> A note\n");
    assert!(text.contains("int x;"), "{text}");
    assert!(text.contains("A note"), "{text}");
    assert!(!text.contains("//"), "{text}");
}

/// An include directive that never closes is just text; in particular, it must not make the scanner
/// run forever (the parse happens on its own thread so that a hang fails the test, not the run)
#[rstest]
#[case::tag_at_end_of_file("include::x.xml[tag=a", "include::x.xml[tag=a")]
#[case::lines_at_end_of_file("include::x.xml[lines=1", "include::x.xml[lines=1")]
#[case::no_attribute_value("include::included.xml[tag=", "include::included.xml[tag=")]
#[case::unclosed_on_its_line(
    "include::x.xml[tag=a\nnext line\n",
    "include::x.xml[tag=a\nnext line"
)]
fn test_unclosed_include_is_text(#[case] adoc: &str, #[case] expected: &str) {
    let (sender, receiver) = std::sync::mpsc::channel();
    let source = adoc.to_string();
    std::thread::spawn(move || {
        let _ = sender.send(parsed_text(&source));
    });
    let text = receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("Parsing did not finish: the scanner is stuck");
    assert_eq!(text, expected);
}

/// A footnote ends at its own `]`, and a footnote that has ended takes nothing after it
#[rstest]
#[case::stray_bracket(
    "A footnote:[plain] here and a stray ] there.\n",
    "</sup> here and a stray ] there.</p>"
)]
#[case::span_after(
    "A footnote:[x] and *b* end.\n",
    "</sup> and <strong>b</strong> end.</p>"
)]
fn test_footnote_ends_at_its_bracket(#[case] adoc: &str, #[case] expected: &str) {
    let html = render_asciidoctor_embedded_from_str(adoc);
    assert!(html.contains(expected), "{html}");
}

/// The `link:` macro, with a URL or a relative target
#[rstest]
#[case::url(
    "See link:http://x.com[the site] now.\n",
    "<p>See <a href=\"http://x.com\">the site</a> now.</p>"
)]
#[case::relative(
    "See link:other.html[other] now.\n",
    "<p>See <a href=\"other.html\">other</a> now.</p>"
)]
#[case::in_footnote("footnote:[see link:x.html[y]] after\n", "</sup> after</p>")]
#[case::bare_url_then_bracket(
    "see http://y.com[ x\n",
    "<p>see <a href=\"http://y.com\" class=\"bare\">http://y.com</a>[ x</p>"
)]
#[case::formatted_text_with_url(
    "link:x.html[*x http://q.com y*]\n",
    "<a href=\"x.html\"><strong>x <a href=\"http://q.com\" class=\"bare\">http://q.com</a> y</strong></a>"
)]
fn test_link_macro(#[case] adoc: &str, #[case] expected: &str) {
    let html = render_asciidoctor_embedded_from_str(adoc);
    assert!(html.contains(expected), "{html}");
}

/// Markup that is never closed is text, at the end of a paragraph as at the end of the document
#[rstest]
#[case::emphasis_before_block("a _unclosed text\n\nb\n", "a _unclosed text")]
#[case::strong_before_block("a *unclosed text\n\nb\n", "a *unclosed text")]
#[case::plus_plus("C++ uses a::b\n", "C++ uses a::b")]
#[case::single_plus("1 + 1 and a+b\n", "1 + 1 and a+b")]
fn test_unclosed_markup_is_text(#[case] adoc: &str, #[case] expected: &str) {
    assert!(
        parsed_text(adoc).starts_with(expected),
        "{}",
        parsed_text(adoc)
    );
}

/// Like Asciidoctor, an apostrophe between letters is a typographic one, in a span or not
#[rstest]
#[case::plain("don't stop\n", "don&#8217;t stop")]
#[case::emphasis("_don't stop_\n", "<em>don&#8217;t stop</em>")]
#[case::strong("*it's here*\n", "<strong>it&#8217;s here</strong>")]
#[case::twice("rock'n'roll\n", "rock&#8217;n&#8217;roll")]
#[case::quotes_untouched("say 'hi' now\n", "say 'hi' now")]
fn test_apostrophes(#[case] adoc: &str, #[case] expected: &str) {
    let html = render_asciidoctor_embedded_from_str(adoc);
    assert!(html.contains(expected), "{html}");
}

/// Windows line endings are line endings
#[test]
fn test_crlf_line_endings() {
    assert_eq!(
        parsed_text("line one\r\nline two\r\n"),
        "line one\nline two"
    );
}

/// As in Asciidoctor, a constrained mark after a space, or before a letter or digit, doesn't
/// close a span, and a span left open inside one that closes is text
#[rstest]
#[case::strong("*a b *c", "*a b *c")]
#[case::emphasis("_a b _c", "_a b _c")]
#[case::monospace("`a b `c", "`a b `c")]
#[case::mark("#a b #c", "#a b #c")]
#[case::closed_later("*a *b* c", "<strong>a *b</strong> c")]
#[case::open_inside_strong("mix *a _b* c", "mix <strong>a _b</strong> c")]
#[case::open_inside_emphasis("mix _a *b_ c", "mix <em>a *b</em> c")]
#[case::open_inside_unconstrained("**a _b** c", "<strong>a _b</strong> c")]
#[case::two_spans("*a* and _b_", "<strong>a</strong> and <em>b</em>")]
#[case::before_non_ascii_letter("*a *é* b", "<strong>a *é</strong> b")]
fn test_spans_that_do_not_close(#[case] adoc: &str, #[case] expected: &str) {
    let html = common::render_asciidoctor_embedded_from_str(adoc);
    common::html::assert_html_eq(
        &common::html::select(&html, "p"),
        &format!("<p>{expected}</p>"),
    );
}

/// A run of marks that never close is text, however long (it overflowed the stack), and quickly
#[test]
fn test_many_unclosed_marks() {
    let adoc = format!("a{}\n", " *x".repeat(20_000));
    let asg = common::parsed_asg_with_timeout(&adoc);
    let inlines = &asg["blocks"][0]["inlines"];
    assert_eq!(inlines.as_array().map(Vec::len), Some(1), "one literal");
}

/// A mark closes the span of its kind however deeply the spans in it are nested, and a mark
/// that can't close one is text when a span of its kind is already open
#[rstest]
#[case::three_deep(
    "*a _b `c` d_ e* f",
    "<strong>a <em>b <code>c</code> d</em> e</strong> f"
)]
#[case::three_deep_unconstrained(
    "**a _b `c` d_ e** f",
    "<strong>a <em>b <code>c</code> d</em> e</strong> f"
)]
#[case::same_kind_again("_a *b _c* d_", "<em>a <strong>b _c</strong> d</em>")]
#[case::file_globs("Files *.txt, *.md and *.rs", "Files *.txt, *.md and *.rs")]
fn test_nested_spans(#[case] adoc: &str, #[case] expected: &str) {
    let html = common::render_asciidoctor_embedded_from_str(adoc);
    common::html::assert_html_eq(
        &common::html::select(&html, "p"),
        &format!("<p>{expected}</p>"),
    );
}
