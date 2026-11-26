pub mod common;
use std::fs;

use common::{
    assert_parsed_doc_matches_expected_asg, assert_parsed_doc_matches_expected_asg_from_str,
    block_outline, block_outline_from_file, block_tree, top_level_block_names,
};
use rstest::rstest;

#[rstest]
#[case::para_single_line("blocks/para-single-line")]
#[case::para_two_lines_space_between("blocks/para-two-paras-line-between")]
#[case::para_internal_line_break("blocks/para-internal-line-break")]
fn test_paragraphs(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[rstest]
#[case::header_title_only("blocks/document-header")]
#[case::header_attrs("blocks/document-attr")]
fn test_document(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[rstest]
#[case::single_unordered("blocks/unordered-list")]
#[case::many_unordered("blocks/unordered-list-many-items")]
#[case::complex_unordered("blocks/unordered-list-complex")]
#[case::complex_unordered_open_block("blocks/unordered-list-complex-open-block")]
#[case::single_ordered("blocks/ordered-list")]
#[case::many_unordered("blocks/ordered-list-many-items")]
#[case::description_simple("blocks/description-list-simple")]
#[case::description_complex("blocks/description-list-complex")]
#[case::unordered_title("blocks/unordered-list-title")]
#[case::ordered_title("blocks/ordered-list-title")]
#[case::description_title("blocks/description-list-title")]
fn test_lists(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

// Note that "Heading1"s are treated as document or 0-levels
#[rstest]
#[case::heading2("== ", 1)]
#[case::heading3("=== ", 2)]
#[case::heading4("==== ", 3)]
#[case::heading5("===== ", 4)]
fn test_simple_heading_sections(#[case] heading_markup: &str, #[case] section_level: usize) {
    let offset = heading_markup.len(); // counts up to the beginning of the text
    let text_start = offset + 1;
    let text_end = offset + "Section Title".len();

    let adoc_str = fs::read_to_string("tests/data/blocks/headings-sections.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("MARKUP", heading_markup);
    let asg_json_str = fs::read_to_string("tests/data/blocks/headings-sections.json")
        .expect("Unable to read asg json test template")
        .replace("91", &text_start.to_string()) // title text start
        .replace("92", &text_end.to_string()) // title text end
        .replace("93", &section_level.to_string()); // "level"
    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[rstest]
#[case::nested_two_in_one("blocks/nested-headings")]
#[case::nested_two_in_one_multiple("blocks/nested-headings-multiple")]
fn test_nested_sections(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[rstest]
#[case::delimited_sidebar("****", "sidebar")]
#[case::delimited_example("====", "example")]
#[case::delimited_quote("____", "quote")]
#[case::delimited_open("--", "open")]
fn test_delimited_blocks_no_meta(#[case] delimiter: &str, #[case] name: &str) {
    let adoc_str = fs::read_to_string("tests/data/blocks/delimited-block.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("****", delimiter);
    let mut asg_json_str = fs::read_to_string("tests/data/blocks/delimited-block.json")
        .expect("Unable to read asg json test template")
        .replace("sidebar", name)
        .replace("****", delimiter);

    // fix location numbering
    if name == "open" {
        asg_json_str = asg_json_str.replace("5, \"col\": 4", "5, \"col\": 2");
    }

    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[rstest]
#[case::delimited_sidebar("****", "sidebar")]
//#[case::delimited_example("====", "example")]
//#[case::delimited_quote("____", "quote")]
//#[case::delimited_open("--", "open")]
fn test_delimited_blocks_no_meta_spaces(#[case] delimiter: &str, #[case] name: &str) {
    let adoc_str = fs::read_to_string("tests/data/blocks/delimited-block-spaced.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("****", delimiter);
    let mut asg_json_str = fs::read_to_string("tests/data/blocks/delimited-block-spaced.json")
        .expect("Unable to read asg json test template")
        .replace("sidebar", name)
        .replace("****", delimiter);

    // fix location numbering
    if name == "open" {
        asg_json_str = asg_json_str.replace("5, \"col\": 4", "5, \"col\": 2");
    }

    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

#[test]
fn test_delimited_comment_at_eof() {
    let fn_pattern = "blocks/comment-eof";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[rstest]
#[case::note("NOTE")]
#[case::note("TIP")]
#[case::note("IMPORTANT")]
#[case::note("CAUTION")]
#[case::note("WARNING")]
fn test_admontions_non_delimited(#[case] admonition: &str) {
    let text_start = admonition.len() + 3;
    let text_end = text_start + 9; // "Notice me!"
    let adoc_str = fs::read_to_string("tests/data/blocks/admonition-inline.adoc")
        .expect("Unable to read asciidoc test template")
        .replace("NOTE", admonition);
    let asg_json_str = fs::read_to_string("tests/data/blocks/admonition-inline.json")
        .expect("Unable to read asg json test template")
        .replace("TEXT_START", &text_start.to_string())
        .replace("TEXT_END", &text_end.to_string())
        .replace("DELIMITER", &format!("{}: ", admonition))
        .replace("ADMONITION", &admonition.to_lowercase());

    assert_parsed_doc_matches_expected_asg_from_str(&adoc_str, &asg_json_str)
}

/// An inline-style admonition ends with its paragraph, wherever that is
#[rstest]
#[case::at_eof("blocks/admonition-inline-eof")]
#[case::before_section("blocks/admonition-inline-then-section")]
fn test_admontions_non_delimited_end(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Block roles (classes) can be applied
fn test_block_meta() {
    let fn_pattern = "blocks/metadata-role-para";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_block_meta_source() {
    let fn_pattern = "blocks/metadata-source-lang";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_block_meta_appropriately_affects_leaf_blocks() {
    let fn_pattern = "blocks/quotes-no-delimiter";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_literal() {
    let fn_pattern = "blocks/literal";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[rstest]
#[case::lines("blocks/verse")]
#[case::stanzas("blocks/verse-stanzas")]
fn test_verse(#[case] fn_pattern: &str) {
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Block roles (classes) can be applied
fn test_block_title_or_label() {
    let fn_pattern = "blocks/block-label";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_block_anchor() {
    let fn_pattern = "blocks/block-anchor";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_block_image() {
    let fn_pattern = "blocks/image-block";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_block_image_alt_text() {
    let fn_pattern = "blocks/image-block-alt-text";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_block_figure() {
    let fn_pattern = "blocks/image-block-figure";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_include() {
    let fn_pattern = "blocks/include";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_include_lines() {
    let fn_pattern = "blocks/include-lines";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_include_tags() {
    let fn_pattern = "blocks/include-tags";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_include_tags_xml() {
    let fn_pattern = "blocks/include-tags-xml";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_include_respects_internal_breaks() {
    let fn_pattern = "blocks/include-respects-internal-breaks";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_table_simple() {
    let fn_pattern = "blocks/table-simple";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_table_star_cols_option_header() {
    let fn_pattern = "blocks/table-cols-star-header-opt";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_table_implicit_header() {
    let fn_pattern = "blocks/table-header-implicit";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_table_implicit_cols() {
    let fn_pattern = "blocks/table-cols-implicit";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_table_with_caption_id() {
    let fn_pattern = "blocks/table-with-caption-id";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_code_callouts() {
    let fn_pattern = "blocks/code-callouts";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_dangling_markup_in_heading() {
    let fn_pattern = "blocks/heading-with-dangling-hash";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

/// A section starts at the top level (here), whatever block comes before it
#[rstest]
#[case::sidebar("****\nSidebar.\n****\n\n== Sec\n\nText.\n", &["sidebar", "section"])]
#[case::example("====\nExample.\n====\n\n== Sec\n\nText.\n", &["example", "section"])]
#[case::quote("[quote]\n____\nQuoted.\n____\n\n== Sec\n\nText.\n", &["quote", "section"])]
#[case::open("--\nOpen.\n--\n\n== Sec\n\nText.\n", &["open", "section"])]
#[case::table("|===\n|a\n|===\n\n== Sec\n\nText.\n", &["table", "section"])]
#[case::admonition_block("[NOTE]\n====\nNote.\n====\n\n== Sec\n\nText.\n", &["admonition", "section"])]
#[case::list("* item\n\n== Sec\n\nText.\n", &["list", "section"])]
fn test_section_after_block_is_not_nested(#[case] adoc: &str, #[case] expected: &[&str]) {
    assert_eq!(top_level_block_names(adoc), expected);
}

/// Sections of a lower level still belong to the section above them
#[test]
fn test_subsections_stay_nested() {
    assert_eq!(
        top_level_block_names("== One\n\n=== Sub\n\nText.\n\n== Two\n\nText.\n"),
        ["section", "section"]
    );
}

/// The same holds inside a section: a subsection is a child of the section, not of the block
/// that precedes it
#[rstest]
#[case::sidebar(
    "== A\n\n****\nx\n****\n\n=== B\n\ntext\n\n== C\n\nmore\n",
    "section(sidebar(paragraph),section(paragraph)) section(paragraph)"
)]
#[case::inline_admonition(
    "== A\n\nNOTE: n\n\n=== B\n\ntext\n",
    "section(admonition(paragraph),section(paragraph))"
)]
#[case::table(
    "== A\n\n|===\n|a\n|===\n\n=== B\n\ntext\n",
    "section(table(tableCell),section(paragraph))"
)]
fn test_subsection_after_block_is_not_nested(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_tree(adoc), expected);
}

/// Like Asciidoctor, plain table cells don't parse list markup (only AsciiDoc `a|` cells would),
/// so a cell that starts with a list marker is text; `|* a` used to fail the whole conversion.
/// So are description list markers and admonition labels (`|x:: y`, `|NOTE: x`)
#[rstest]
#[case::list_markers(
    "|===\n|* a | 1. b\n|===\n",
    "table(tableCell \"* a \", tableCell \" 1. b\")"
)]
#[case::description_list_markers(
    "|===\n|c;; d |x:: y\n|===\n",
    "table(tableCell \"c;; d \", tableCell \"x:: y\")"
)]
#[case::admonition_label(
    "|===\n|NOTE: x |b\n|===\n",
    "table(tableCell \"NOTE: x \", tableCell \"b\")"
)]
fn test_list_markers_in_table_cells_are_text(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// A heading's title is its own line, and like Asciidoctor, a heading can't interrupt a
/// paragraph
#[rstest]
#[case::text_after_heading("== Head\nText\n", "section \"Head\"(paragraph \"Text\")")]
#[case::heading_after_text("Text\n== B\n", "paragraph \"Text\n== B\"")]
#[case::heading_after_blank_line("Text\n\n== B\n", "paragraph \"Text\" section \"B\"")]
fn test_heading_lines(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// Like Asciidoctor, a delimiter interrupts a paragraph, and the text after a closing delimiter
/// is a new paragraph (`Text` then `----` is a setext title in Asciidoctor, which we don't support,
/// so these use other delimiters)
#[rstest]
#[case::literal_after_text("Text\n....\nx\n....\n", "paragraph \"Text\" literal \"\nx\n\"")]
#[case::sidebar_after_text("Text\n****\nx\n****\n", "paragraph \"Text\" sidebar(paragraph \"x\")")]
#[case::text_after_listing(
    "Para\n\n----\nx\n----\nmore\n",
    "paragraph \"Para\" listing \"\nx\n\" paragraph \"more\""
)]
#[case::text_after_literal("....\nx\n....\nmore\n", "literal \"\nx\n\" paragraph \"more\"")]
#[case::text_after_sidebar("****\nx\n****\nmore\n", "sidebar(paragraph \"x\") paragraph \"more\"")]
#[case::in_section(
    "== S\n\n....\nx\n....\nmore\n",
    "section \"S\"(literal \"\nx\n\", paragraph \"more\")"
)]
fn test_delimiters_end_paragraphs(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// A delimited block inside another one, and the text after each
#[rstest]
#[case::sidebar_in_example(
    "====\nouter\n****\ninner\n****\nafter inner\n====\nafter\n",
    "example(paragraph \"outer\", sidebar(paragraph \"inner\"), paragraph \"after inner\") paragraph \"after\""
)]
#[case::open_in_example(
    "====\nouter\n--\ninner\n--\n====\nafter\n",
    "example(paragraph \"outer\", open(paragraph \"inner\")) paragraph \"after\""
)]
#[case::only_a_sidebar(
    "====\n****\ninner\n****\n====\nafter\n",
    "example(sidebar(paragraph \"inner\")) paragraph \"after\""
)]
fn test_nested_delimited_blocks(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// A delimited block in an included file that opens on the same line number as the block the
/// include is in is a block of its own (the lines of the two files are counted separately)
#[test]
fn test_delimited_block_in_include() {
    assert_eq!(
        block_outline_from_file("blocks/include-delimited.adoc"),
        "paragraph \"x\" example(paragraph \"first\", paragraph \"a\", sidebar(paragraph \"sidebar in the include\"), paragraph \"last\") paragraph \"after\""
    );
}

/// Generated section ids are for the backends and for xrefs, not part of the graph: the ASG
/// (TCK) output has no `id` for a section without an explicit one. An xref to it gets the title.
#[test]
fn test_generated_section_ids_stay_out_of_the_graph() {
    let asg = common::parsed_asg("== My Section\n\nSee <<_my_section>>.\n");
    let section = &asg["blocks"][0];
    assert!(section.get("id").is_none(), "{section}");
    assert!(
        !section.to_string().contains("_my_section\\\""),
        "{section}"
    );
    let xref = &section["blocks"][0]["inlines"][1];
    assert_eq!(xref["target"], "_my_section");
    assert_eq!(xref["inlines"][0]["value"], "My Section", "{xref}");
}

/// An xref with its own text (`<<id,text>>`) keeps it, and its target is the id alone
#[test]
fn test_xref_custom_text() {
    let asg = common::parsed_asg("== A\n\nSee <<_a,the A section>>.\n");
    let xref = &asg["blocks"][0]["blocks"][0]["inlines"][1];
    assert_eq!(xref["target"], "_a", "{xref}");
    assert_eq!(xref["inlines"][0]["value"], "the A section", "{xref}");
}

/// Found by the fuzz run (tests/fuzz_tests.rs): markup on the last line of a file, with no newline
/// after it, made the scanner run past the end of the input (a panic) or loop forever
#[rstest]
#[case::example_delimiter("====", "example")]
#[case::example_delimiter_after_text("x\n\n====", "paragraph \"x\" example")]
#[case::block_anchor("text\n\n[[para-id]]", "paragraph \"text\"")]
#[case::attribute_entry("text\n\n:cat: George", "paragraph \"text\"")]
#[case::header_attribute_entry("= Title\n:foo: bar", "")]
#[case::equals_signs_without_space("====x", "paragraph \"====x\"")]
#[case::inline_image_name("image:", "paragraph \"image:\"")]
#[case::block_image_without_brackets("image::pixel", "paragraph \"image::pixel\"")]
#[case::block_image_unclosed("image::ab.png[Alt", "paragraph \"image::ab.png[Alt\"")]
#[case::inline_image_without_brackets("An image:c&d.png", "paragraph \"An image:c&d.png\"")]
// without its brackets on its own line, an image macro doesn't reach a later line's `]`
#[case::image_then_bracket_later("image::x.png\nlater ]\n", "paragraph \"image::x.png\nlater ]\"")]
#[case::inline_style_unclosed("a [.role", "paragraph \"a [.role\"")]
fn test_markup_at_end_of_file(#[case] adoc: &str, #[case] expected: &str) {
    let asg = common::parsed_asg_with_timeout(adoc);
    let outline = match asg["blocks"].as_array() {
        Some(_) => block_outline(adoc),
        None => String::new(),
    };
    assert_eq!(outline, expected);
}

/// Found by the fuzz run: a table whose first row isn't all on one line stopped the whole process
/// ("Error creating table"). As in Asciidoctor, the cells on the first line are the columns
#[rstest]
#[case::one_cell_per_line("|===\n|a\n|b\n|===\n", "table(tableCell \"a\", tableCell \"b\")")]
#[case::first_line_one_cell(
    "|===\n|Name\n\n|a\n|1\n|===\n",
    "table(tableCell \"Name\", tableCell \"a\", tableCell \"1\")"
)]
fn test_tables_with_one_cell_on_the_first_line(#[case] adoc: &str, #[case] expected: &str) {
    let asg = common::parsed_asg_with_timeout(adoc);
    assert_eq!(asg["blocks"][0]["metadata"]["attributes"]["cols"], "1");
    assert_eq!(block_outline(adoc), expected);
}

/// An include that can't be read (a directory, here) is skipped with a warning; it used to stop
/// the whole process
#[test]
fn test_unreadable_include_is_skipped() {
    assert_eq!(
        block_outline_from_file("blocks/include-unreadable.adoc"),
        "paragraph \"before\" paragraph \"after\""
    );
}

/// Found by the full fuzz run: inputs that made the parser panic, and what Asciidoctor makes of
/// them
#[rstest]
#[case::lone_colon(":", "paragraph \":\"")]
#[case::attribute_reference_at_end(":cat: George\n\n{", "paragraph \"{\"")]
#[case::ampersand_at_end("bar &", "paragraph \"bar &\"")]
#[case::include_without_brackets("include::x.adoc]\n", "paragraph \"include::x.adoc]\"")]
#[case::stray_quote_in_attributes("[role=\"foo\"\"]\npara\n", "paragraph \"para\"")]
#[case::pipe_in_list_item("* |s cool\n", "list(* \"|s cool\")")]
#[case::pipe_outside_table("|not a table\n", "paragraph \"|not a table\"")]
fn test_inputs_found_by_fuzzing(#[case] adoc: &str, #[case] expected: &str) {
    common::parsed_asg_with_timeout(adoc);
    assert_eq!(block_outline(adoc), expected);
}

/// Like Asciidoctor, a `+` line outside a list is text: it starts a paragraph of its own, which
/// takes the lines after it (it used to stop the conversion, or be dropped)
#[rstest]
#[case::after_paragraph("Para\n+\nafter\n", "paragraph \"Para\" paragraph \"+\nafter\"")]
#[case::after_blank_line("Para\n\n+\nafter\n", "paragraph \"Para\" paragraph \"+\nafter\"")]
#[case::alone(
    "Para\n\n+\n\nafter\n",
    "paragraph \"Para\" paragraph \"+\" paragraph \"after\""
)]
#[case::first_line("+\nafter\n", "paragraph \"+\nafter\"")]
#[case::twice("+\n+\nx\n", "paragraph \"+\" paragraph \"+\nx\"")]
#[case::after_heading("== Head\n+\nafter\n", "section \"Head\"(paragraph \"+\nafter\")")]
#[case::after_listing(
    "----\nx\n----\n+\nafter\n",
    "listing \"\nx\n\" paragraph \"+\nafter\""
)]
#[case::in_example(
    "====\na\n+\nb\n====\n",
    "example(paragraph \"a\", paragraph \"+\nb\")"
)]
#[case::in_block_in_list(
    "* x\n+\n====\na\n+\nb\n====\n",
    "list(* \"x\"(example(paragraph \"a\", paragraph \"+\nb\")))"
)]
#[case::after_header("= T\n\n+\nafter\n", "paragraph \"+\nafter\"")]
#[case::second_in_list("* a\n+\n+\nb\n", "list(* \"a\"(paragraph \"+\nb\"))")]
#[case::in_listing("----\na\n+\nb\n----\n", "listing \"\na\n+\nb\n\"")]
#[case::author_line("= T\n+\n\ny\n", "paragraph \"y\"")]
#[case::revision_line("= T\nJane Doe\n+\n\ny\n", "paragraph \"y\"")]
#[case::in_verse("[verse]\n____\na\n+\nb\n____\n", "verse \"a\n+\nb\"")]
#[case::title_between_in_list("* a\n+\n.T\n+\nb\n", "list(* \"a\"(paragraph \"b\"))")]
#[case::anchor_between_in_list("* a\n+\n[[x]]\n+\nb\n", "list(* \"a\"(paragraph \"b\"))")]
#[case::comment_between_in_list("* a\n+\n// c\n+\nb\n", "list(* \"a\"(paragraph \"b\"))")]
#[case::attribute_between_in_list("* a\n+\n:foo: bar\n+\nb\n", "list(* \"a\"(paragraph \"b\"))")]
fn test_plus_line_outside_list_is_text(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// A `+` in an included file is treated like one in the document
#[test]
fn test_plus_line_in_include() {
    assert_eq!(
        block_outline_from_file("blocks/include-plus.adoc"),
        "section \"H\"(paragraph \"Para\", paragraph \"+\nafter\", list(* \"a\"(paragraph \"+\nb\")))"
    );
}

/// A `+` line in a table doesn't stop the conversion either (which cell its text lands in is
/// another matter; see the roadmap)
#[test]
fn test_plus_line_in_table() {
    let adoc = "|===\n|a\n+\n|b\n|===\n";
    assert_eq!(top_level_block_names(adoc), ["table"]);
    assert!(common::parsed_text(adoc).contains('+'));
}

/// A `= Title` after content is a level 0 section, as in Asciidoctor (it used to stop the
/// conversion)
#[rstest]
#[case::after_paragraph(
    "x\n\n= One\n\na\n",
    "paragraph \"x\" section \"One\"(paragraph \"a\")"
)]
#[case::after_list(
    "* a\n\n= One\n\nmore\n",
    "list(* \"a\") section \"One\"(paragraph \"more\")"
)]
#[case::holds_sections(
    "x\n\n= One\n\n== Two\n\nb\n\n= Three\n\nc\n",
    "paragraph \"x\" section \"One\"(section \"Two\"(paragraph \"b\")) section \"Three\"(paragraph \"c\")"
)]
#[case::after_section(
    "== A\n\n=== B\n\n= One\n\na\n",
    "section \"A\"(section \"B\") section \"One\"(paragraph \"a\")"
)]
#[case::second_title("= T\n\n= One\n\na\n", "section \"One\"(paragraph \"a\")")]
fn test_level_zero_sections(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// Comments, attribute entries and blank lines can come before the document title
#[rstest]
#[case::comment("// c\n= Title\n\ntext\n")]
#[case::comment_block("////\nc\n////\n= Title\n\ntext\n")]
#[case::attribute(":a: b\n= Title\n\ntext\n")]
#[case::blank_lines("\n\n= Title\n\ntext\n")]
fn test_title_below_the_first_line(#[case] adoc: &str) {
    let asg = common::parsed_asg(adoc);
    assert_eq!(asg["header"]["title"][0]["value"], "Title", "{asg}");
    assert_eq!(block_outline(adoc), "paragraph \"text\"");
}

/// A title can use the attributes set above it
#[test]
fn test_attribute_reference_in_title() {
    let adoc = ":product: Widget\n= {product} Guide\nJane Doe\n\ntext\n";
    let asg = common::parsed_asg(adoc);
    let title = asg["header"]["title"].to_string();
    assert!(title.contains("Widget") && title.contains("Guide"), "{asg}");
    assert_eq!(asg["header"]["authors"][0]["fullname"], "Jane Doe", "{asg}");
    assert_eq!(block_outline(adoc), "paragraph \"text\"");
}

/// Attributes above a title that isn't on the first line don't move to the first paragraph
#[test]
fn test_attributes_above_the_title_are_ignored() {
    let asg = common::parsed_asg("[[top]]\n= Title\n\ntext\n");
    assert_eq!(asg["header"]["title"][0]["value"], "Title", "{asg}");
    assert!(asg["blocks"][0].get("metadata").is_none(), "{asg}");
}

/// Like Asciidoctor, `::` (or `;;`) with no term before it is text (the text after it used to be
/// lost)
#[rstest]
#[case::after_list_item("* a\n:: b\n", "list(* \"a\n:: b\")")]
#[case::after_description("A:: a\n:: b\n", "dlist(\"A\":: \"a\n:: b\")")]
#[case::after_list_and_blank_line("* a\n\n:: b\n", "list(* \"a\") paragraph \":: b\"")]
#[case::after_paragraph("text\n:: b\n", "paragraph \"text\n:: b\"")]
#[case::alone(":: b\n", "paragraph \":: b\"")]
#[case::semicolons("* a\n;; b\n", "list(* \"a\n;; b\")")]
#[case::colon_term("::::: b\n", "dlist(\":\":: \"b\")")]
fn test_description_marker_without_a_term_is_text(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// Attribute lists that used to make the parser panic: a lone quote as a value, and non-ASCII
/// text
#[rstest]
#[case::lone_role_quote("[role=\"]\ntext\n", "paragraph \"text\"")]
#[case::lone_quote("[foo=\"]\ntext\n", "paragraph \"text\"")]
#[case::unclosed_quote("[role=\"é]\ntext\n", "paragraph \"text\"")]
#[case::non_ascii("[aé]\ntext\n", "paragraph \"text\"")]
#[case::non_ascii_cols("[cols=\"é\"]\n|===\n|a\n|===\n", "table(tableCell \"a\")")]
#[case::in_macro("a image:x.png[role=\"] b\n", "paragraph \"a  b\"")]
fn test_attribute_lists_that_used_to_panic(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// An unclosed quote is part of the value, as in Asciidoctor, rather than costing it its last
/// character
#[test]
fn test_unclosed_quote_in_attribute_value() {
    let asg = common::parsed_asg("[role=\"foo]\ntext\n");
    assert_eq!(asg["blocks"][0]["metadata"]["roles"][0], "\"foo", "{asg}");
}

/// Like Asciidoctor, a `leveloffset` never stops the conversion: a signed value is relative, a
/// value that isn't a number resets the offset, and a level out of range is brought into it
#[rstest]
#[case::below_zero_title(
    "x\n\n:leveloffset: -1\n\n= One\n\na\n",
    "paragraph \"x\" section \"One\"(paragraph \"a\")",
    0
)]
#[case::to_zero(
    "x\n\n:leveloffset: -1\n\n== One\n",
    "paragraph \"x\" section \"One\"",
    0
)]
#[case::too_high("x\n\n:leveloffset: 9\n\n== A\n", "paragraph \"x\" section \"A\"", 5)]
#[case::huge("x\n\n:leveloffset: 200\n\n== A\n", "paragraph \"x\" section \"A\"", 5)]
#[case::not_a_number(
    "x\n\n:leveloffset: 2\n:leveloffset: y\n\n== A\n",
    "paragraph \"x\" section \"A\"",
    1
)]
#[case::empty(
    "x\n\n:leveloffset: 2\n:leveloffset:\n\n== A\n",
    "paragraph \"x\" section \"A\"",
    1
)]
#[case::unset(
    "x\n\n:leveloffset: 2\n:leveloffset!:\n\n== A\n",
    "paragraph \"x\" section \"A\"",
    1
)]
#[case::relative(
    "x\n\n:leveloffset: +1\n:leveloffset: +1\n\n== A\n",
    "paragraph \"x\" section \"A\"",
    3
)]
fn test_level_offsets(#[case] adoc: &str, #[case] expected: &str, #[case] level: u64) {
    assert_eq!(block_outline(adoc), expected);
    assert_eq!(common::parsed_asg(adoc)["blocks"][1]["level"], level);
}

/// Newlines are only kept apart from the text around them in description lists; a paragraph after
/// one is like any other
#[test]
fn test_paragraph_after_description_list() {
    let asg = common::parsed_asg("A:: a\n\nPara one\nline two\n");
    let inlines = &asg["blocks"][1]["inlines"];
    assert_eq!(inlines.as_array().map(Vec::len), Some(1), "{inlines}");
    assert_eq!(inlines[0]["value"], "Para one\nline two");
}

/// As in Asciidoctor, text on the lines after a cell is that cell's, every `|` in a table starts
/// a cell, and `\|` is a pipe
#[rstest]
#[case::next_line(
    "|===\n|a\nmore\n|b\n|===\n",
    "table(tableCell \"a\nmore\", tableCell \"b\")"
)]
#[case::after_a_row(
    "|===\n|a |b\nmore\n|c |d\n|===\n",
    "table(tableCell \"a \", tableCell \"b\nmore\", tableCell \"c \", tableCell \"d\")"
)]
#[case::after_blank_line(
    "|===\n|a\n\nmore\n|b\n|===\n",
    "table(tableCell \"a\nmore\", tableCell \"b\")"
)]
#[case::mid_line(
    "|===\n|a\nmore|b\n|===\n",
    "table(tableCell \"a\nmore\", tableCell \"b\")"
)]
#[case::list_marker(
    "|===\n|a\n* b\n|c\n|===\n",
    "table(tableCell \"a\n* b\", tableCell \"c\")"
)]
#[case::heading(
    "|===\n|a\n== b\n|c\n|===\n",
    "table(tableCell \"a\n== b\", tableCell \"c\")"
)]
#[case::no_spaces(
    "|===\n|Name|Value\n|a |b\n|===\n",
    "table(tableCell \"Name\", tableCell \"Value\", tableCell \"a \", tableCell \"b\")"
)]
#[case::empty_cell(
    "|===\n|a||c\n|===\n",
    "table(tableCell \"a\", tableCell, tableCell \"c\")"
)]
#[case::escaped_pipe(
    "|===\n|a\\|x |b\n|===\n",
    "table(tableCell \"a|x \", tableCell \"b\")"
)]
#[case::pipe_after_table("|===\n|a\n|===\n\nx|y\n", "table(tableCell \"a\") paragraph \"x|y\"")]
fn test_table_cells(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// As in Asciidoctor, the first row is a header when it is directly under the delimiter and a
/// blank line follows it
#[rstest]
#[case::blank_after_row("|===\n|a |b\n\n|c |d\n|===\n", true)]
#[case::one_row_then_blank("|===\n|a |b\n\n|===\n", true)]
#[case::one_row("|===\n|a |b\n|===\n", false)]
#[case::no_blank_line("|===\n|a |b\n|c |d\n|===\n", false)]
#[case::blank_before("|===\n\n|a |b\n\n|c |d\n|===\n", false)]
#[case::row_continues("|===\n|a |b\nmore\n\n|c |d\n|===\n", false)]
fn test_implicit_table_header(#[case] adoc: &str, #[case] header: bool) {
    let asg = common::parsed_asg(adoc);
    let options = &asg["blocks"][0]["metadata"]["options"];
    let has_header = options
        .as_array()
        .is_some_and(|options| options.iter().any(|option| option == "header"));
    assert_eq!(has_header, header, "{asg}");
}

/// An ID or role on an admonition is the admonition's, not its content's
#[rstest]
#[case::inline_form("[[i]]\nNOTE: text\n")]
#[case::paragraph_form("[[i]]\n[NOTE]\ntext\n")]
#[case::block_form("[[i]]\n[NOTE]\n====\ntext\n====\n")]
#[case::anchor_below("[NOTE]\n[[i]]\n====\ntext\n====\n")]
#[case::attribute_list("[id=i]\nNOTE: text\n")]
fn test_admonition_ids(#[case] adoc: &str) {
    let asg = common::parsed_asg(adoc);
    let admonition = &asg["blocks"][0];
    assert_eq!(admonition["name"], "admonition", "{asg}");
    assert_eq!(admonition["metadata"]["attributes"]["id"], "i", "{asg}");
    let content = &admonition["blocks"][0];
    assert!(content.get("metadata").is_none(), "{asg}");
}

/// A `+` on the last line, with no newline after it, is still a continuation or a line break
#[rstest]
#[case::after_item("* a\n+", "list(* \"a\")")]
#[case::after_paragraph("x\n+", "paragraph \"x\" paragraph \"+\"")]
#[case::line_break("a +", "paragraph \"a \"")]
fn test_plus_on_the_last_line(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// A table delimiter inside a verbatim or comment block doesn't leave the text after it being
/// read as if it were in a table
#[rstest]
#[case::listing("----\n|===\n----\n\nx | *b*\n")]
#[case::literal("....\n|===\n....\n\nx | *b*\n")]
#[case::comment("////\n|===\n////\n\nx | *b*\n")]
#[case::pass("++++\n|===\n++++\n\nx | *b*\n")]
fn test_table_delimiter_in_verbatim_block(#[case] adoc: &str) {
    let html = common::render_asciidoctor_embedded_from_str(adoc);
    assert!(html.contains("x | <strong>b</strong>"), "{html}");
}

/// Rows included into a table are split like the table's own
#[test]
fn test_table_rows_in_include() {
    assert_eq!(
        block_outline_from_file("blocks/include-table.adoc"),
        "table(tableCell \"a \", tableCell \"b\", tableCell \"c \", tableCell \"d\", tableCell \"e \", tableCell \"f\")"
    );
}

/// Cell specifiers (spans, alignment and styles) aren't supported, but they aren't text either
#[rstest]
#[case::span("|===\n|a\n2+|b\n|===\n", "table(tableCell \"a\", tableCell \"b\")")]
#[case::row_span("|===\n|a .2+|b\n|===\n", "table(tableCell \"a \", tableCell \"b\")")]
#[case::alignment(
    "|===\n|a ^|b >.^|c\n|===\n",
    "table(tableCell \"a \", tableCell \"b \", tableCell \"c\")"
)]
#[case::style(
    "|===\n|a h|b\na|c\n|===\n",
    "table(tableCell \"a \", tableCell \"b\", tableCell \"c\")"
)]
#[case::words(
    "|===\n|plan a\n|vitamin|b\n|===\n",
    "table(tableCell \"plan a\", tableCell \"vitamin\", tableCell \"b\")"
)]
fn test_cell_specifiers(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

/// The first row goes on while its cells do, and `noheader` turns its header off
#[rstest]
#[case::first_row_runs_on("|===\n|a\nmore |b\n|c |d\n|===\n", "2", false)]
#[case::noheader("[%noheader]\n|===\n|a |b\n\n|c |d\n|===\n", "2", false)]
#[case::noheader_option("[options=\"noheader\"]\n|===\n|a |b\n\n|c |d\n|===\n", "2", false)]
fn test_first_table_row(#[case] adoc: &str, #[case] cols: &str, #[case] header: bool) {
    let asg = common::parsed_asg(adoc);
    let metadata = &asg["blocks"][0]["metadata"];
    assert_eq!(metadata["attributes"]["cols"], cols, "{asg}");
    let has_header = metadata["options"]
        .as_array()
        .is_some_and(|options| options.iter().any(|option| option == "header"));
    assert_eq!(has_header, header, "{asg}");
}

/// Like Asciidoctor, block markup inside a table cell is the cell's text. Delimiters used to
/// stop the conversion, or take the rest of the document into a block
#[rstest]
#[case::example("====")]
#[case::listing("----")]
#[case::listing_pair("----\nx\n----")]
#[case::literal("....")]
#[case::passthrough("++++")]
#[case::comment("////")]
#[case::open("--")]
#[case::sidebar("****")]
#[case::quote("____")]
#[case::breaks("'''\n<<<")]
#[case::attribute_list("[.role]\n[source]")]
#[case::anchor("[[anchor]]")]
#[case::attribute_entry(":foo: bar")]
#[case::block_image("image::x.png[]")]
#[case::block_title_and_lists(".T\n<1> c\n- d\n1. e")]
#[case::heading("====== f")]
fn test_block_markup_in_table_cell_is_text(#[case] markup: &str) {
    let adoc = format!("|===\n|a\n{markup}\n|b\n|===\n\nafter *c*\n");
    assert_eq!(
        block_outline(&adoc),
        format!("table(tableCell \"a\n{markup}\", tableCell \"b\") paragraph \"after c\"")
    );
}

/// The same for a cell's first line, and for lines that come from an include
#[test]
fn test_delimiters_that_start_table_cells_are_text() {
    assert_eq!(
        block_outline("|===\n|a ====\n|----\n|===\n\nafter\n"),
        "table(tableCell \"a ====\", tableCell \"----\") paragraph \"after\""
    );
    assert_eq!(
        block_outline_from_file("blocks/include-table-text.adoc"),
        "table(tableCell \"a\ninc cell\n====\", tableCell \"b\") paragraph \"after\""
    );
}

/// Six `=` signs make the deepest section (they stopped the conversion), and seven are text
#[rstest]
#[case::first("====== A\n\ntext\n", "section \"A\"(paragraph \"text\")")]
#[case::nested("== A\n\n====== B\n", "section \"A\"(section \"B\")")]
#[case::after_text("x\n====== A\n", "paragraph \"x\n====== A\"")]
#[case::seven("======= A\n", "paragraph \"======= A\"")]
#[case::in_listing("----\n====== A\n----\n", "listing \"\n====== A\n\"")]
fn test_deepest_headings(#[case] adoc: &str, #[case] expected: &str) {
    assert_eq!(block_outline(adoc), expected);
}

#[test]
fn test_six_equals_signs_are_level_five() {
    let asg = common::parsed_asg("x\n\n====== A\n");
    assert_eq!(asg["blocks"][1]["level"], 5, "{asg}");
}
