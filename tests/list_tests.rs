//! List parsing: markers, item text, nesting, continuations, and where lists end. Expected
//! results follow Asciidoctor (2.0.17); see "Lists" in ROADMAP.adoc. Tests for behavior that
//! later steps of the lists rework will add are ignored until that step lands.
pub mod common;

use common::{block_outline, block_outline_with_timeout};
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

/// Like Asciidoctor, the indentation of a lazy continuation line is not part of the text
#[test]
#[ignore = "lists: strip the indentation of lazy lines"]
fn test_list_item_lazy_line_indentation() {
    assert_eq!(
        block_outline("* one\n  continued text\n* two\n"),
        "list(* \"one\ncontinued text\", * \"two\")"
    );
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
#[case::comment_separator(
    ". one\n. two\n\n//-\n\n. new list\n",
    "list(. \"one\", . \"two\") list(. \"new list\")"
)]
// Until step 7: Asciidoctor keeps "* a" as text in the paragraph (see
// test_list_interrupting_paragraph)
#[case::paragraph_then_list("Text\n* a\n", "paragraph \"Text\" list(* \"a\")")]
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

/// Until step 5 (blank lines between items), the blank line ends the list; it used to be an
/// error ("invalid block continuation; no previous block")
#[test]
fn test_list_continuation_then_blank_line() {
    assert_eq!(
        block_outline("* one\n+\npara\n\n* two\n"),
        "list(* \"one\"(paragraph \"para\")) list(* \"two\")"
    );
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
#[ignore = "lists step 5: blank lines between items, ancestor continuation"]
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
#[ignore = "lists step 6: description lists in the list context"]
fn test_list_after_dlist_item_and_blank_line() {
    assert_eq!(
        block_outline_with_timeout("Term:: def\n\n* item\n"),
        "dlist(\"Term\":: \"def\"(list(* \"item\")))"
    );
}

/// A list item can't interrupt a paragraph, and a heading can't interrupt an item's text
#[rstest]
#[case::paragraph_then_marker("Text\n* a\n", "paragraph \"Text\n* a\"")]
#[case::item_then_heading("* a\n== B\n\ntext\n", "list(* \"a\n== B\") paragraph \"text\"")]
#[ignore = "lists step 7: lists after headings and paragraphs"]
fn test_list_interrupting_paragraph(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline_with_timeout(adoc), expected);
}
