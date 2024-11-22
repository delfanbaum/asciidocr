use std::fs;

use asciidocr::docx::render_docx;
use asciidocr::parser::Parser;
use asciidocr::scanner::Scanner;
use tempfile::NamedTempFile;

#[test]
fn first_test_for_sanity() {
    let adoc_fn = "blocks/para-single-line.adoc";
    let test_dir = "tests/data/";
    let parsed_asg = Parser::new().parse(Scanner::new(
        &fs::read_to_string(&format!("{}{}", test_dir, adoc_fn)).expect("Unable to find adoc"),
    ));
    let temp_docx = NamedTempFile::new().unwrap();
    match render_docx(&parsed_asg, &temp_docx.path()) {
        Ok(_) => assert!(true),
        Err(_) => assert!(false),
    }
}
