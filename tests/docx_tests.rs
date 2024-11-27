use std::fs;
use std::path::PathBuf;

use asciidocr::backends::docx::render_docx;
use asciidocr::parser::Parser;
use asciidocr::scanner::Scanner;
use tempfile::NamedTempFile;

#[test]
fn first_test_for_sanity() {
    let test_dir = PathBuf::from("tests/data/");
    let adoc_fn = "blocks/para-single-line.adoc";
    let parsed_asg = Parser::new(test_dir.join(adoc_fn)).parse(Scanner::new(
        &fs::read_to_string(test_dir.join(adoc_fn)).expect("Unable to find adoc"),
    ));
    let temp_docx = NamedTempFile::new().unwrap();
    match render_docx(&parsed_asg, &temp_docx.path()) {
        Ok(_) => assert!(true),
        Err(_) => assert!(false),
    }
}
