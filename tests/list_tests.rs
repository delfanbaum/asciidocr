//! List parsing: markers, item text, nesting, continuations, and where lists end. Expected
//! results follow Asciidoctor (2.0.17); see "Lists" in ROADMAP.adoc. Tests for behavior that
//! later steps of the lists rework will add are ignored until that step lands.
pub mod common;

use common::{block_outline, block_outline_with_timeout, parsed_asg};
use rstest::rstest;

#[rstest]
#[case::star("* a\n* b\n", "list(* \"a\", * \"b\")")]
#[case::dot(". a\n. b\n", "list(. \"a\", . \"b\")")]
#[case::dash("- a\n- b\n", "list(- \"a\", - \"b\")")]
#[case::numbered("1. a\n2. b\n", "list(1. \"a\", 2. \"b\")")]
#[case::indented("  * a\n  * b\n", "list(* \"a\", * \"b\")")]
#[case::tab_indented("\t- a\n", "list(- \"a\")")]
#[case::double_star("** a\n** b\n", "list(** \"a\", ** \"b\")")]
#[case::double_dot(".. a\n.. b\n", "list(.. \"a\", .. \"b\")")]
#[case::triple_dot("... a\n", "list(... \"a\")")]
#[case::five_stars("***** a\n", "list(***** \"a\")")]
#[case::lower_alpha("a. x\nb. y\n", "list(a. \"x\", b. \"y\")")]
#[case::upper_alpha("A. x\nB. y\n", "list(A. \"x\", B. \"y\")")]
#[case::lower_roman("i) x\nii) y\niii) z\n", "list(i) \"x\", ii) \"y\", iii) \"z\")")]
#[case::upper_roman("I) x\nII) y\n", "list(I) \"x\", II) \"y\")")]
#[case::letter_after_dot(". x\na. y\n. z\n", "list(. \"x\"(list(a. \"y\")), . \"z\")")]
#[case::roman_after_letter("a. x\ni) y\nb. z\n", "list(a. \"x\"(list(i) \"y\")), b. \"z\")")]
// like Asciidoctor: a line that starts with an initial is a list item
#[case::initial("A. Lincoln was president.\n", "list(A. \"Lincoln was president.\")")]
// `i.` is a letter, and `ii.` is no marker at all
#[case::roman_with_dot("i. x\nii. y\n", "list(i. \"x\nii. y\")")]
fn test_list_markers(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// Markup that looks like a marker, but isn't one
#[rstest]
#[case::ellipsis_at_line_start("...and then\n", "paragraph \"&#8230;and then\"")]
#[case::strong_at_line_start("**bold** text\n", "paragraph \"bold text\"")]
#[case::number_without_space("1.5 cups\n", "paragraph \"1.5 cups\"")]
#[case::dash_without_space("-x\n", "paragraph \"-x\"")]
#[case::block_title(".Title\n* a\n", "list(* \"a\")")]
fn test_list_marker_lookalikes(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// An item's text belongs to that item, whatever follows it
#[rstest]
#[case::multiline("* a\nb\n", "list(* \"a\nb\")")]
#[case::continued_item_then_item(
    "* one\n+\nmore\n* two\n",
    "list(* \"one\"(paragraph \"more\"), * \"two\")"
)]
#[case::two_continuations(
    "* one\n+\nmore\n+\nevenmore\n* two\n",
    "list(* \"one\"(paragraph \"more\", paragraph \"evenmore\"), * \"two\")"
)]
#[case::continued_listing_then_item(
    "* one\n+\n----\ncode\n----\n* two\n",
    "list(* \"one\"(listing \"\ncode\n\"), * \"two\")"
)]
#[case::text_after_attached_block(
    "* a\n+\n----\nx\n----\nmore text\n* b\n",
    "list(* \"a\"(listing \"\nx\n\", paragraph \"more text\"), * \"b\")"
)]
#[case::multiline_attached_paragraph(
    "* a\n+\nline one\nline two\n",
    "list(* \"a\"(paragraph \"line one\nline two\"))"
)]
#[case::continued_open_block(
    "* item\n+\n--\nfoo\n--\n",
    "list(* \"item\"(open(paragraph \"foo\")))"
)]
#[case::continued_open_block_then_item(
    "* item\n+\n--\nfoo\n--\n* two\n",
    "list(* \"item\"(open(paragraph \"foo\")), * \"two\")"
)]
fn test_list_item_text(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// Like Asciidoctor, the indentation of an item's lazy lines is not part of its text, while a
/// paragraph's (attached or not) keeps it
#[rstest]
#[case::list_item(
    "* one\n  continued text\n* two\n",
    "list(* \"one\ncontinued text\", * \"two\")"
)]
#[case::dlist_item("A:: a\n  more\n", "dlist(\"A\":: \"a\nmore\")")]
#[case::attached_paragraph("* a\n+\npara\n  more\n", "list(* \"a\"(paragraph \"para\n  more\"))")]
#[case::paragraph("Para\n  indented\n", "paragraph \"Para\n  indented\"")]
fn test_list_item_lazy_line_indentation(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// What ends a list, and what comes before it
#[rstest]
#[case::block_without_continuation(
    "* one\n[source,rust]\n----\nfn x() {}\n----\n* two\n",
    "list(* \"one\") listing \"\nfn x() {}\n\" list(* \"two\")"
)]
#[case::heading_then_list("== Head\n* a\n* b\n", "section \"Head\"(list(* \"a\", * \"b\"))")]
#[case::paragraph_between(
    "* one\n\nA paragraph.\n\n* two\n",
    "list(* \"one\") paragraph \"A paragraph.\" list(* \"two\")"
)]
#[case::literal_without_continuation(
    "* one\n....\nlit\n....\n",
    "list(* \"one\") literal \"\nlit\n\""
)]
#[case::comment_separator(
    ". one\n. two\n\n//-\n\n. new list\n",
    "list(. \"one\", . \"two\") list(. \"new list\")"
)]
fn test_list_ends(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// A list directly after a description list item belongs to that item
#[test]
fn test_list_after_dlist_item() {
    assert_eq!(
        block_outline("Term:: def\n* item\n* item2\nTerm2:: def2\n"),
        "dlist(\"Term\":: \"def\"(list(* \"item\", * \"item2\")), \"Term2\":: \"def2\")"
    );
}

/// Any kind of block can be attached to an item, and it stays in the item, even when the document
/// ends before the block does
#[rstest]
#[case::admonition_paragraph("* a\n+\nNOTE: hi\n", "list(* \"a\"(admonition(paragraph \"hi\")))")]
#[case::admonition_paragraph_then_item(
    "* a\n+\nNOTE: hi\n* b\n",
    "list(* \"a\"(admonition(paragraph \"hi\")), * \"b\")"
)]
#[case::admonition_paragraph_blank_line_item(
    "* a\n+\nNOTE: hi\n\n* b\n",
    "list(* \"a\"(admonition(paragraph \"hi\")), * \"b\")"
)]
#[case::admonition_block(
    "* a\n* b\n+\n[NOTE]\n====\ntext\n====\n",
    "list(* \"a\", * \"b\"(admonition(example(paragraph \"text\"))))"
)]
#[case::unterminated_listing("* a\n+\n----\n", "list(* \"a\"(listing \"\n\"))")]
#[case::unterminated_example_with_list(
    "* a\n+\n====\n* x\n",
    "list(* \"a\"(example(list(* \"x\"))))"
)]
#[case::image("* a\n+\nimage::x.png[]\n* b\n", "list(* \"a\"(image), * \"b\")")]
#[case::thematic_break("* a\n+\n'''\n* b\n", "list(* \"a\"(break), * \"b\")")]
#[case::page_break("* a\n+\n<<<\n* b\n", "list(* \"a\"(break), * \"b\")")]
fn test_attached_blocks(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline_with_timeout(adoc), expected);
}

/// An attached paragraph takes the attributes above it, like any other paragraph
#[rstest]
#[case::role("* a\n+\n[role=special]\npara\n", "roles", "[\"special\"]")]
#[case::id("* a\n+\n[[para-id]]\npara\n", "attributes", "{\"id\":\"para-id\"}")]
fn test_attached_paragraph_metadata(#[case] adoc: &str, #[case] key: &str, #[case] expected: &str) {
    let asg = parsed_asg(adoc);
    let attached = &asg["blocks"][0]["items"][0]["blocks"][0];
    assert_eq!(
        attached["metadata"][key].to_string(),
        expected,
        "{attached}"
    );
}

/// A styled paragraph attached to an item keeps its style (`[verse]` paragraphs aren't
/// supported yet; see the roadmap)
#[rstest]
#[case::quote("* a\n+\n[quote]\nfoo\n", "list(* \"a\"(quote(paragraph \"foo\")))")]
fn test_attached_styled_paragraphs(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// An attribute or anchor line ends a paragraph, so a list after it is a list
#[rstest]
#[case::anchor("text\n[[id]]\n* a\n", "paragraph \"text\" list(* \"a\")")]
#[case::attributes("text\n[.role]\n* a\n", "paragraph \"text\" list(* \"a\")")]
#[case::paragraph("text\n[[id]]\nmore\n", "paragraph \"text\" paragraph \"more\"")]
fn test_attribute_lines_end_paragraphs(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// Lists nest by marker identity: a marker that isn't open yet starts a deeper list, and one
/// that is open closes back to its level
#[rstest]
#[case::nested(
    "* parent\n** nested\n** nested two\n* parent two\n",
    "list(* \"parent\"(list(** \"nested\", ** \"nested two\")), * \"parent two\")"
)]
#[case::three_levels(
    "* parent\n** child\n*** grandchild\n** child two\n* parent two\n",
    "list(* \"parent\"(list(** \"child\"(list(*** \"grandchild\")), ** \"child two\")), * \"parent two\")"
)]
#[case::ordered_depths(
    ". one\n.. one-a\n... one-a-i\n. two\n",
    "list(. \"one\"(list(.. \"one-a\"(list(... \"one-a-i\")))), . \"two\")"
)]
#[case::mixed(
    ". one\n.. one-a\n* mixed\n. two\n",
    "list(. \"one\"(list(.. \"one-a\"(list(* \"mixed\")))), . \"two\")"
)]
#[case::mixed_siblings("* a\n. b\n* c\n", "list(* \"a\"(list(. \"b\")), * \"c\")")]
#[case::dash_under_star(
    "* a\n- b\n- c\n* d\n",
    "list(* \"a\"(list(- \"b\", - \"c\")), * \"d\")"
)]
#[case::numbered_under_dot(". a\n1. b\n2. c\n", "list(. \"a\"(list(1. \"b\", 2. \"c\")))")]
#[case::shorter_marker_nests(
    "** starts deep\n*** deeper\n* shallow\n",
    "list(** \"starts deep\"(list(*** \"deeper\"(list(* \"shallow\")))))"
)]
#[case::lazy_line_then_nested("* a\nlazy\n** b\n", "list(* \"a\nlazy\"(list(** \"b\")))")]
#[case::continuation_on_nested(
    "* a\n** b\n+\nattached to b\n* c\n",
    "list(* \"a\"(list(** \"b\"(paragraph \"attached to b\"))), * \"c\")"
)]
#[case::continuation_then_nested_list(
    "* parent\n+\n. one\n. two\n",
    "list(* \"parent\"(list(. \"one\", . \"two\")))"
)]
fn test_nested_lists(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline_with_timeout(adoc), expected);
}

/// Blank lines between items don't end a list; the next line decides. A blank line before a
/// `+` attaches the block to the top-level item
#[rstest]
#[case::blank_between_items("* one\n* two\n\n* three\n", "list(* \"one\", * \"two\", * \"three\")")]
#[case::two_blank_lines("* one\n\n\n* two\n", "list(* \"one\", * \"two\")")]
#[case::blank_before_nested("* a\n\n** b\n\n* c\n", "list(* \"a\"(list(** \"b\")), * \"c\")")]
#[case::blank_then_other_marker("* a\n\n. b\n", "list(* \"a\"(list(. \"b\")))")]
#[case::continuation_blank_item(
    "* one\n+\npara\n\n* two\n",
    "list(* \"one\"(paragraph \"para\"), * \"two\")"
)]
#[case::ancestor_continuation(
    "* a\n** b\n\n+\nattached to a\n",
    "list(* \"a\"(list(** \"b\"), paragraph \"attached to a\"))"
)]
#[case::ancestor_continuation_three_levels(
    "* a\n** b\n*** c\n\n+\none blank\n",
    "list(* \"a\"(list(** \"b\"(list(*** \"c\"))), paragraph \"one blank\"))"
)]
#[case::ancestor_continuation_two_blank_lines(
    "* a\n** b\n*** c\n\n\n+\ntwo blanks\n",
    "list(* \"a\"(list(** \"b\"(list(*** \"c\"))), paragraph \"two blanks\"))"
)]
#[case::ancestor_continuation_top_level(
    "* a\n\n+\nsolo ancestor\n",
    "list(* \"a\"(paragraph \"solo ancestor\"))"
)]
fn test_blank_lines_between_items(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline_with_timeout(adoc), expected);
}

/// A delimited block attached to an item has its own lists; the outer list resumes after it
#[test]
fn test_list_scopes() {
    assert_eq!(
        block_outline("* a\n+\n--\npara\n\n* inner\n--\n* b\n"),
        "list(* \"a\"(open(paragraph \"para\", list(* \"inner\"))), * \"b\")"
    );
}

/// A list after a description list item belongs to it, even after a blank line
#[test]
fn test_list_after_dlist_item_and_blank_line() {
    assert_eq!(
        block_outline_with_timeout("Term:: def\n\n* item\n"),
        "dlist(\"Term\":: \"def\"(list(* \"item\")))"
    );
}

/// Description lists follow the same rules as other lists: their items take the lines that
/// follow, `+` attaches a block, blank lines don't end them, and lists nest in their items
#[rstest]
#[case::lazy_line(
    "A:: a\nmore a\nB:: b\n",
    "dlist(\"A\":: \"a\nmore a\", \"B\":: \"b\")"
)]
#[case::continuation(
    "A:: a\n+\npara\nB:: b\n",
    "dlist(\"A\":: \"a\"(paragraph \"para\"), \"B\":: \"b\")"
)]
#[case::blank_between_items("A:: a\n\nB:: b\n", "dlist(\"A\":: \"a\", \"B\":: \"b\")")]
#[case::ancestor_continuation(
    "A:: a\n\n\nB:: b\n\n+\npara\n",
    "dlist(\"A\":: \"a\", \"B\":: \"b\"(paragraph \"para\"))"
)]
#[case::text_after_blank_line(
    "A:: a\n\nText\nmore text\n",
    "dlist(\"A\":: \"a\") paragraph \"Text\nmore text\""
)]
#[case::block_without_continuation(
    "A:: a\n....\nlit\n....\n",
    "dlist(\"A\":: \"a\") literal \"\nlit\n\""
)]
#[case::two_terms("A::\nB:: b\n", "dlist(\"A\" \"B\":: \"b\")")]
#[case::two_terms_blank_line("A::\n\nB:: b\n", "dlist(\"A\" \"B\":: \"b\")")]
#[case::three_terms("A::\nB::\nC:: c\n", "dlist(\"A\" \"B\" \"C\":: \"c\")")]
#[case::terms_after_described_item(
    "A:: a\nB::\nC:: c\n",
    "dlist(\"A\":: \"a\", \"B\" \"C\":: \"c\")"
)]
#[case::terms_in_list_item("* x\nA::\nB:: b\n", "list(* \"x\"(dlist(\"A\" \"B\":: \"b\")))")]
#[case::indented_term("  A:: a\n", "dlist(\"A\":: \"a\")")]
#[case::indented_term_after_paragraph(
    "Intro\n\n  A:: a\n",
    "paragraph \"Intro\" dlist(\"A\":: \"a\")"
)]
#[case::in_list_item("* x\nA:: a\n", "list(* \"x\"(dlist(\"A\":: \"a\")))")]
#[case::list_item_after_term(
    "A:: a\n* x\nB:: b\n** y\n",
    "dlist(\"A\":: \"a\"(list(* \"x\")), \"B\":: \"b\"(list(** \"y\")))"
)]
fn test_description_lists(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline_with_timeout(adoc), expected);
}

/// Each description list marker (`::`, `:::`, `::::`, `;;`) is its own list, and they nest by
/// marker like other lists
#[rstest]
#[case::semicolons(
    "A:: a\nB;; b\nC;; c\nD:: d\n",
    "dlist(\"A\":: \"a\"(dlist(\"B\":: \"b\", \"C\":: \"c\")), \"D\":: \"d\")"
)]
#[case::colons(
    "A:: a\nB::: b\nC:::: c\nD:: d\n",
    "dlist(\"A\":: \"a\"(dlist(\"B\":: \"b\"(dlist(\"C\":: \"c\")))), \"D\":: \"d\")"
)]
#[case::semicolons_first("A;; a\nB:: b\n", "dlist(\"A\":: \"a\"(dlist(\"B\":: \"b\")))")]
#[case::triple_colon_alone("A::: a\n", "dlist(\"A\":: \"a\")")]
#[case::after_blank_line("A:: a\n\nB;; b\n", "dlist(\"A\":: \"a\"(dlist(\"B\":: \"b\")))")]
// a term with another marker nests, rather than being another term
#[case::term_then_other_marker("A::\nB;; b\n", "dlist(\"A\":: \"\"(dlist(\"B\":: \"b\")))")]
#[case::terms_same_marker("A;;\nB;; b\n", "dlist(\"A\" \"B\":: \"b\")")]
#[case::in_list_item("* x\nT;; d\n", "list(* \"x\"(dlist(\"T\":: \"d\")))")]
#[case::colons_in_text("a::b::c\n", "paragraph \"a::b::c\"")]
// like Asciidoctor, a description on the line after its term loses its indentation
#[case::description_on_next_line("Term::\n  desc\n", "dlist(\"Term\":: \"desc\")")]
fn test_nested_description_lists(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline_with_timeout(adoc), expected);
}

/// The marker as written is in the graph, on the list and on its items
#[test]
fn test_description_list_markers_in_the_graph() {
    let asg = parsed_asg("A:: a\nB;; b\n");
    let outer = &asg["blocks"][0];
    let inner = &outer["items"][0]["blocks"][0];
    assert_eq!(outer["marker"], "::");
    assert_eq!(outer["items"][0]["marker"], "::");
    assert_eq!(inner["marker"], ";;");
    assert_eq!(inner["items"][0]["marker"], ";;");
}

/// A term that ends its line (`Term::`) counts that line, so the blocks after it have the right
/// line numbers
#[test]
fn test_lines_after_term_line() {
    let asg = parsed_asg("Term::\ndesc\n\nPara\n");
    assert_eq!(
        asg["blocks"][1]["location"][0]["line"], 4,
        "{}",
        asg["blocks"][1]
    );
}

/// A list item can't interrupt a paragraph, and a heading can't interrupt an item's text
#[rstest]
#[case::paragraph_then_marker("Text\n* a\n", "paragraph \"Text\n* a\"")]
#[case::block_title_line("text\n.Title\n* a\n", "paragraph \"text\n.Title\n* a\"")]
#[case::item_then_heading("* a\n== B\n\ntext\n", "list(* \"a\n== B\") paragraph \"text\"")]
fn test_list_interrupting_paragraph(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline_with_timeout(adoc), expected);
}
