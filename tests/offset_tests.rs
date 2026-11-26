pub mod common;

use std::env::current_dir;

use asciidocr::parser::Parser;
use asciidocr::scanner::Scanner;
use common::assert_parsed_doc_matches_expected_asg;
use rstest::rstest;

#[test]
fn test_offset_absolute() {
    let fn_pattern = "blocks/level-offset-absolute";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_offset_relative() {
    let fn_pattern = "blocks/level-offset-relative";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

#[test]
fn test_offset_as_inline_attribute() {
    let fn_pattern = "blocks/level-offset-attributes";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
}

/// Like Asciidoctor, an offset that puts a level out of range doesn't stop the conversion: the
/// level is brought into range
#[rstest]
#[case::too_big(":leveloffset: +7\n\ntext\n\n== Pass me\n", 5)]
#[case::too_small(":leveloffset: -3\n\ntext\n\n== Pass me\n", 0)]
fn test_offset_out_of_range(#[case] adoc: &str, #[case] level: u64) {
    assert_eq!(common::parsed_asg(adoc)["blocks"][1]["level"], level);
}

/// An include's `leveloffset` is only for the file it includes
#[test]
fn test_include_offset_ends_with_the_include() {
    let asg = common::parsed_asg_from_file("blocks/include-offset.adoc");
    let levels: Vec<_> = (1..4).map(|i| asg["blocks"][i]["level"].as_u64()).collect();
    assert_eq!(levels, [Some(2), Some(2), Some(1)], "{asg}");
}

#[test]
fn test_offest_level_0() {
    let adoc_str = String::from(":leveloffset: +1\n\n= Pass me\n\nOkay");
    let result = Parser::new(current_dir().unwrap()).parse(Scanner::new(&adoc_str));
    assert!(result.is_ok())
}
