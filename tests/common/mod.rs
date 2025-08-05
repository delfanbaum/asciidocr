use std::env;
use std::{fs, path::PathBuf};

use asciidocr::{backends::htmls::render_htmlbook, parser::Parser, scanner::Scanner};
use assert_json_diff::assert_json_eq;
use image::RgbImage;
use serde_json::{Value, json};
use tempfile::NamedTempFile;

/// Given a pattern "parent/filename" inside tests/data, assert that the filename.adoc produces
/// the expected abstract syntax graph found in filename.json
pub fn assert_parsed_doc_matches_expected_asg(adoc_fn: &str, asg_json_fn: &str) {
    let test_dir = PathBuf::from("tests/data/");
    let parsed_asg = Parser::new_no_target_resolution(test_dir.join(adoc_fn))
        .parse(Scanner::new(
            &fs::read_to_string(test_dir.join(adoc_fn)).expect("Unable to find adoc"),
        ))
        .expect("Unable to parse input file");
    let asg_as_json = json!(parsed_asg);

    let expected_asg: Value = serde_json::from_str(
        &fs::read_to_string(test_dir.join(asg_json_fn)).expect("Unable to find asg json"),
    )
    .unwrap();
    assert_json_eq!(asg_as_json, expected_asg);
}

/// Given an asciidoc string and an expected abstract syntax graph json string, assert that the
/// parser produces the correct json from the asciidoc
pub fn assert_parsed_doc_matches_expected_asg_from_str(adoc_str: &str, asg_json_str: &str) {
    let origin = env::current_dir().unwrap();
    let parsed_asg = json!(
        Parser::new_no_target_resolution(origin)
            .parse(Scanner::new(adoc_str))
            .expect("Unable to parse input file")
    );
    let expected_asg: Value = serde_json::from_str(asg_json_str).unwrap();
    assert_json_eq!(parsed_asg, expected_asg);
}

/// Ignoring whitespace (because that's less important for now), ensure that our tags, ordering,
/// blocks, inlines, etc., are correct
pub fn assert_rendered_htmlbook_matches_expected(adoc_fn: &str, html_fn: &str) {
    let test_dir = PathBuf::from("tests/data/");
    let mut rendered_html = render_htmlbook(
        &Parser::new_no_target_resolution(test_dir.join(adoc_fn))
            .parse(Scanner::new(
                &fs::read_to_string(test_dir.join(adoc_fn)).expect("Unable to find adoc"),
            ))
            .expect("Unable to parse input file"),
    )
    .expect("Unable to render HTML from document");
    rendered_html.retain(|c| !c.is_whitespace());
    let mut expected_html =
        fs::read_to_string(test_dir.join(html_fn)).expect("Unable to read expectd html file");
    expected_html.retain(|c| !c.is_whitespace());
    assert_eq!(rendered_html, expected_html);
}

pub fn generate_temp_image() -> NamedTempFile {
    let img_path = NamedTempFile::with_suffix(".png").unwrap();
    let img = RgbImage::new(32, 32);
    img.save(&img_path.path()).expect("Error creating image");
    img_path
}
