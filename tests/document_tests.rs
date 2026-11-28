use std::{fs, path::PathBuf};

use asciidocr::{parser::Parser, scanner::Scanner};
use common::{
    assert_parsed_doc_matches_expected_asg, assert_parsed_doc_matches_expected_asg_from_str,
};
use rstest::rstest;

pub mod common;

#[test]
/// Test that we pull reference text from the target into reference as an inline
fn test_reference_test_insertion() {
    let fn_pattern = "documents/references-titles";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Test that we pull reference text from the target into reference as an inline but when it's an
/// include
fn test_reference_test_in_include() {
    let fn_pattern = "documents/references-include-fig";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Test that we pull reference text from the target into reference as an inline but when it's an
/// include
fn test_nested_includes() {
    let fn_pattern = "documents/nested-includes";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Test that we pull reference text from the target into reference as an inline but when it's an
/// include
fn test_title_only() {
    let fn_pattern = "documents/title-only";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
/// Test dangling spans in title
fn test_title_dangling_span() {
    let fn_pattern = "documents/title-dangling-span";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
///Smoke test for "have we covered enough" -- will be added to as we go along and do not panic
fn test_targeted_coverage() {
    let adoc_fn = "tests/data/documents/minimum.adoc";
    let _ = Parser::new(PathBuf::from(adoc_fn)).parse(Scanner::new(
        &fs::read_to_string(adoc_fn).expect("Unable to find adoc"),
    ));
}

/// The AsciiDoc TCK's header test (tests/block/header/attribute-entries-below-title), with its
/// expected ASG as it is
#[test]
fn test_tck_attribute_entries_below_title() {
    assert_parsed_doc_matches_expected_asg_from_str(
        "= Document Title\n:icons: font\n:toc:\n",
        r#"{
  "name": "document",
  "type": "block",
  "attributes": { "icons": "font", "toc": "" },
  "header": {
    "title": [
      {
        "name": "text",
        "type": "string",
        "value": "Document Title",
        "location": [{ "line": 1, "col": 3 }, { "line": 1, "col": 16 }]
      }
    ],
    "location": [{ "line": 1, "col": 1 }, { "line": 3, "col": 5 }]
  },
  "location": [{ "line": 1, "col": 1 }, { "line": 3, "col": 5 }]
}"#,
    );
}

/// The author and revision lines under the title: authors go in the header (as the ASG schema
/// has them); the revision has no place in the ASG; and the attributes they imply (`author`,
/// `revnumber`, ...) work in references, but aren't listed with the document's own attributes
#[test]
fn test_author_and_revision_lines() {
    let asg = common::parsed_asg(
        "= Document Title\nJane Q. Doe <jane@example.com>; John Smith\nv1.2, 2026-01-15: A remark\n:description: About things\n\nBy {author}, version {revnumber} of {revdate}.\n",
    );
    let header = &asg["header"];
    assert_eq!(header["title"][0]["value"], "Document Title", "{header}");
    assert_eq!(
        header["authors"],
        serde_json::json!([
            {
                "fullname": "Jane Q. Doe",
                "initials": "JQD",
                "firstname": "Jane",
                "middlename": "Q.",
                "lastname": "Doe",
                "address": "jane@example.com"
            },
            {
                "fullname": "John Smith",
                "initials": "JS",
                "firstname": "John",
                "lastname": "Smith"
            }
        ])
    );
    assert_eq!(
        asg["attributes"],
        serde_json::json!({ "description": "About things" })
    );
    assert_eq!(
        body_text(
            "= T\nJane Q. Doe <jane@example.com>\nv1.2, 2026-01-15\n\nBy {author}, version {revnumber} of {revdate}.\n"
        ),
        "By Jane Q. Doe, version 1.2 of 2026-01-15."
    );
}

/// The forms of an author line and a revision line, as Asciidoctor reads them
#[rstest]
#[case::one_name("= T\nCher\n", "Cher", "C")]
#[case::underscored_name("= T\nMary_Ann Smith\n", "Mary Ann Smith", "MS")]
#[case::more_than_three_names("= T\nA B C D\n", "A B C D", "A")]
fn test_author_names(#[case] adoc: &str, #[case] fullname: &str, #[case] initials: &str) {
    let asg = common::parsed_asg(adoc);
    let author = &asg["header"]["authors"][0];
    assert_eq!(author["fullname"], fullname, "{author}");
    assert_eq!(author["initials"], initials, "{author}");
}

#[rstest]
#[case::number_only("= T\nJane Doe\nv1.0\n\n{revnumber}|{revdate}\n", "1.0|{revdate}")]
#[case::date_only(
    "= T\nJane Doe\n2026-01-15\n\n{revnumber}|{revdate}\n",
    "{revnumber}|2026-01-15"
)]
#[case::number_and_date(
    "= T\nJane Doe\nv1.0, 2026-01-15\n\n{revnumber}|{revdate}\n",
    "1.0|2026-01-15"
)]
#[case::remark("= T\nJane Doe\nv1.0, 2026-01-15: Fixed\n\n{revremark}\n", "Fixed")]
fn test_revision_line(#[case] adoc: &str, #[case] text: &str) {
    assert_eq!(body_text(adoc), text);
}

/// The text of a document's first block (after its header)
fn body_text(adoc: &str) -> String {
    let asg = common::parsed_asg(adoc);
    asg["blocks"][0]["inlines"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|inline| inline["value"].as_str())
        .collect()
}

/// `:author:` and `:email:` attribute entries in the header stand in for an author line
#[test]
fn test_author_attribute_entries() {
    let adoc =
        "= T\n:author: Jane Q. Doe\n:email: jane@example.com\n\nBy {author} ({authorinitials}).\n";
    let asg = common::parsed_asg(adoc);
    assert_eq!(asg["header"]["authors"][0]["fullname"], "Jane Q. Doe");
    assert_eq!(asg["header"]["authors"][0]["address"], "jane@example.com");
    assert_eq!(body_text(adoc), "By Jane Q. Doe (JQD).");
}

/// Words are counted as LibreOffice counts them (the numbers in the first group are what it
/// reports for the same text)
#[rstest]
#[case::plain("one two three", 3)]
#[case::em_dash("one—two three", 3)]
#[case::spaced_em_dash("one — two", 2)]
#[case::en_dash("1914–1918 war", 3)]
#[case::hyphen("well-known e-mail", 2)]
#[case::lone_punctuation("a & b ! ?", 5)]
#[case::url("see http://example.com/a-b?c=d now", 3)]
#[case::quote_after_dash("“wait—” she said", 4)]
#[case::no_break_space("a\u{a0}b", 2)]
#[case::zero_width_space("a\u{200b}b", 1)]
#[case::chinese("中文字符 test", 5)]
#[case::chinese_punctuation("中文，中文。", 6)]
#[case::kana("コーヒー", 4)]
#[case::hangul("한국어 단어", 5)]
#[case::among_latin("abc中def", 3)]
// the same rules, for what AsciiDoc makes of the source
#[case::blocks("one\n\ntwo\n\nthree\n\n* four\n* five\n", 5)]
#[case::title_and_sections("= Title Here\n\n== A Section\n\nbody\n", 5)]
#[case::dash_replacement("one -- two", 2)]
#[case::apostrophe("it's don't", 2)]
#[case::formatting("a *bold*, _word_ and **in**side", 5)]
#[case::character_reference("a&#8212;b &amp; c&nbsp;d", 5)]
#[case::line_break("one +\ntwo", 2)]
#[case::link_text("see link:a.html[the page] now", 4)]
#[case::bare_link("see http://example.com now", 3)]
#[case::footnote("word.footnote:[A note.] more", 4)]
#[case::inline_image("a image:i.png[alt text] b", 2)]
#[case::description_list("term one:: the description", 4)]
#[case::table("|===\n|one two |three\n|===\n", 3)]
#[case::block_titles(".A title\n====\ninside\n====\n", 3)]
#[case::listing("----\nlet x = 1;\n----\n", 4)]
#[case::passthrough("++++\n<p class=\"a b\">raw words</p>\n++++\n", 2)]
#[case::comments("// not this\none\n\n////\nnor this\n////\n", 1)]
#[case::dash_replacement_unspaced("one--two", 2)]
#[case::dash_replacement_at_end("wait--\n\nthen", 2)]
#[case::inline_passthrough("a pass:[<b class=\"x y\">bold</b>] word", 3)]
#[case::xref_to_a_title("== Two Words\n\nsee <<_two_words>>", 5)]
#[case::several_terms("one::\ntwo:: three\n", 3)]
#[case::image_caption(".A caption\nimage::i.png[alt text]\n", 2)]
#[case::list_title(".A title\n* item\n", 3)]
fn test_word_count(#[case] adoc: &str, #[case] expected: usize) {
    let graph = Parser::default()
        .parse(Scanner::new(adoc))
        .expect("Unable to parse");
    assert_eq!(graph.word_count(), expected);
}
