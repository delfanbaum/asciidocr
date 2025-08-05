pub mod common;

#[cfg(feature = "docx")]
#[cfg(test)]
mod docx_tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::common::generate_temp_image;
    use asciidocr::backends::docx::render_docx;
    use asciidocr::parser::Parser;
    use asciidocr::scanner::Scanner;
    use tempfile::NamedTempFile;

    #[test]
    fn first_test_for_sanity() {
        let test_dir = PathBuf::from("tests/data/");
        let adoc_fn = "blocks/para-single-line.adoc";
        let parsed_asg = Parser::new(test_dir.join(adoc_fn))
            .parse(Scanner::new(
                &fs::read_to_string(test_dir.join(adoc_fn)).expect("Unable to find adoc"),
            ))
            .expect("Failed to parse document");
        let temp_docx = NamedTempFile::new().unwrap();
        match render_docx(&parsed_asg, &temp_docx.path()) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn test_no_todo_failures_for_expected_elements() {
        let test_dir = PathBuf::from("tests/data/");
        let adoc_fn = "documents/minimum.adoc";
        let temp_docx = NamedTempFile::new().unwrap();
        let temp_image = generate_temp_image();
        // mock out an image for the purposes of the test
        let mut adoc_str = fs::read_to_string(test_dir.join(adoc_fn)).expect("Unable to find adoc");
        adoc_str = adoc_str.replace(
            "example_image.png",
            &temp_image.path().display().to_string(),
        );

        let parsed_asg = Parser::new(test_dir.join(adoc_fn))
            .parse(Scanner::new(&adoc_str))
            .expect("Failed to parse document");
        match render_docx(&parsed_asg, &temp_docx.path()) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
    }
}
