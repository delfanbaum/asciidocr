pub mod common;

#[cfg(feature = "docx")]
#[cfg(test)]
mod docx_tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::common::{
        assert_rendered_docx_runs_match_expected, generate_temp_image, rendered_docx_part,
    };
    use asciidocr::backends::docx::render_docx;
    use asciidocr::parser::Parser;
    use asciidocr::scanner::Scanner;
    use rstest::rstest;
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
        render_docx(&parsed_asg, temp_docx.path()).expect("Failed to render docx");
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
        render_docx(&parsed_asg, temp_docx.path()).expect("Failed to render docx");
    }

    /// docx-rs writes East Asian layout compatibility flags by default. Word and LibreOffice
    /// respond to them by adding extra space at every bold/italic run boundary, so we strip them.
    #[test]
    fn settings_have_no_east_asian_layout_flags() {
        let adoc = fs::read_to_string("tests/data/docx/emphasis-spacing.adoc").unwrap();
        let settings = rendered_docx_part(&adoc, "word/settings.xml");
        assert!(!settings.contains("useFELayout"), "{settings}");
        assert!(
            !settings.contains("balanceSingleByteDoubleByteWidth"),
            "{settings}"
        );
        // We still want the rest of the compat block
        assert!(settings.contains("compatibilityMode"), "{settings}");
    }

    /// The page number is right-aligned with paragraph justification. A text frame (`framePr`)
    /// also works in Word, but LibreOffice ignores its alignment and puts the number on the left.
    #[test]
    fn header_page_number_is_right_justified() {
        let header = rendered_docx_part("Some text.", "word/header1.xml");
        let doc = roxmltree::Document::parse(&header).unwrap();
        let para = doc
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "p")
            .expect("header has no paragraph");
        assert!(
            para.descendants().any(|n| n.is_element()
                && n.tag_name().name() == "instrText"
                && n.text() == Some("PAGE")),
            "{header}"
        );
        let jc = para
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "jc")
            .and_then(|n| n.attributes().find(|a| a.name() == "val"))
            .map(|a| a.value());
        assert_eq!(jc, Some("right"), "{header}");
        assert!(!header.contains("framePr"), "{header}");
        assert!(!header.contains("<w:tab"), "{header}");
    }

    #[rstest]
    #[case::emphasis_spacing("docx/emphasis-spacing")]
    #[case::nested_spans("docx/nested-spans")]
    #[case::sub_sup_with_bold("docx/sub-sup-with-bold")]
    #[case::multiline_spans("docx/multiline-spans")]
    fn test_inline_runs(#[case] fn_pattern: &str) {
        let adoc_fn = format!("{}.adoc", fn_pattern);
        let json_fn = format!("{}.json", fn_pattern);
        assert_rendered_docx_runs_match_expected(&adoc_fn, &json_fn)
    }

    #[test]
    fn styles_are_defined_only_once() {
        let adoc = fs::read_to_string("tests/data/docx/emphasis-spacing.adoc").unwrap();
        let styles = rendered_docx_part(&adoc, "word/styles.xml");
        let doc = roxmltree::Document::parse(&styles).unwrap();
        let mut ids: Vec<&str> = doc
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "style")
            .filter_map(|n| {
                n.attributes()
                    .find(|a| a.name() == "styleId")
                    .map(|a| a.value())
            })
            .collect();
        let total = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(total, ids.len(), "duplicate styleIds in styles.xml");
    }
}
