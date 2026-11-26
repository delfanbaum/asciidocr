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

    /// A paragraph of the document: its style, text, and list numbering (id and level)
    #[derive(Debug, PartialEq)]
    struct DocxParagraph {
        style: Option<String>,
        text: String,
        numbering: Option<(String, String)>,
    }

    fn val(node: roxmltree::Node, tag: &str) -> Option<String> {
        node.descendants()
            .find(|n| n.is_element() && n.tag_name().name() == tag)
            .and_then(|n| n.attributes().find(|a| a.name() == "val"))
            .map(|a| a.value().to_string())
    }

    fn docx_paragraphs(adoc: &str) -> Vec<DocxParagraph> {
        let document = rendered_docx_part(adoc, "word/document.xml");
        let doc = roxmltree::Document::parse(&document).unwrap();
        doc.descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "p")
            .map(|p| DocxParagraph {
                style: val(p, "pStyle"),
                text: p
                    .descendants()
                    .filter(|n| n.is_element() && n.tag_name().name() == "t")
                    .filter_map(|n| n.text())
                    .collect(),
                numbering: val(p, "numId").zip(val(p, "ilvl")),
            })
            .filter(|p| !p.text.is_empty())
            .collect()
    }

    /// The number format of a numbering's level, from numbering.xml
    fn number_format(adoc: &str, num_id: &str, level: &str) -> Option<String> {
        let numbering = rendered_docx_part(adoc, "word/numbering.xml");
        let doc = roxmltree::Document::parse(&numbering).unwrap();
        let attr = |n: roxmltree::Node, name: &str| {
            n.attributes()
                .find(|a| a.name() == name)
                .map(|a| a.value().to_string())
        };
        let num = doc.descendants().find(|n| {
            n.is_element()
                && n.tag_name().name() == "num"
                && attr(*n, "numId").as_deref() == Some(num_id)
        })?;
        let abstract_id = val(num, "abstractNumId")?;
        let abstract_num = doc.descendants().find(|n| {
            n.is_element()
                && n.tag_name().name() == "abstractNum"
                && attr(*n, "abstractNumId") == Some(abstract_id.clone())
        })?;
        let lvl = abstract_num.children().find(|n| {
            n.is_element()
                && n.tag_name().name() == "lvl"
                && attr(*n, "ilvl").as_deref() == Some(level)
        })?;
        val(lvl, "numFmt")
    }

    fn numbering_of(paragraphs: &[DocxParagraph], text: &str) -> (String, String) {
        paragraphs
            .iter()
            .find(|p| p.text == text)
            .and_then(|p| p.numbering.clone())
            .unwrap_or_else(|| panic!("\"{text}\" is not numbered: {paragraphs:?}"))
    }

    /// A nested list's items are numbered at its depth, and the outer list goes on after it
    #[test]
    fn nested_lists_are_numbered_at_their_depth() {
        let paragraphs = docx_paragraphs("* a\n** b\n*** c\n* d\n");
        let (a_id, a_level) = numbering_of(&paragraphs, "a");
        let (b_id, b_level) = numbering_of(&paragraphs, "b");
        let (_, c_level) = numbering_of(&paragraphs, "c");
        assert_eq!(
            (a_level.as_str(), b_level.as_str(), c_level.as_str()),
            ("0", "1", "2")
        );
        assert_ne!(a_id, b_id);
        assert_eq!(numbering_of(&paragraphs, "d"), (a_id, a_level));
        assert_eq!(
            number_format("* a\n** b\n", &b_id, "1").as_deref(),
            Some("bullet")
        );
    }

    /// Every list has its own numbering, so ordered lists start at 1 (docx-rs defines a
    /// numbering of its own with id 1, so ours must not use it)
    #[test]
    fn every_list_has_its_own_numbering() {
        let adoc = ". one\n. two\n\n//-\n\n. again\n\n//-\n\n* x\n\n//-\n\n* y\n";
        let paragraphs = docx_paragraphs(adoc);
        let ids: Vec<String> = ["one", "two", "again", "x", "y"]
            .iter()
            .map(|text| numbering_of(&paragraphs, text).0)
            .collect();
        assert_eq!(ids[0], ids[1]);
        assert_ne!(ids[1], ids[2]);
        assert!(!ids.contains(&"1".to_string()), "{ids:?}");
        assert_eq!(
            number_format(adoc, &ids[2], "0").as_deref(),
            Some("decimal")
        );
        assert_eq!(number_format(adoc, &ids[3], "0").as_deref(), Some("bullet"));
        assert_eq!(number_format(adoc, &ids[4], "0").as_deref(), Some("bullet"));
    }

    /// Like Asciidoctor, ordered lists are numbered by marker (`..` is lower alpha), not depth
    #[rstest]
    #[case::dot(". a\n", "a", "0", "decimal")]
    #[case::double_dot_nested(". a\n.. b\n", "b", "1", "lowerLetter")]
    #[case::triple_dot("... a\n", "a", "0", "lowerRoman")]
    #[case::numbered_nested("* a\n1. b\n", "b", "1", "decimal")]
    fn ordered_numbering_follows_the_marker(
        #[case] adoc: &str,
        #[case] text: &str,
        #[case] level: &str,
        #[case] format: &str,
    ) {
        let (id, item_level) = numbering_of(&docx_paragraphs(adoc), text);
        assert_eq!(item_level, level);
        assert_eq!(number_format(adoc, &id, level).as_deref(), Some(format));
    }

    /// Blocks attached to an item are indented paragraphs, and the next item is still an item
    #[test]
    fn items_after_attached_blocks_stay_in_the_list() {
        let paragraphs = docx_paragraphs("* a\n+\nmore\n+\n--\nopen\n--\n* b\n");
        let texts: Vec<&str> = paragraphs.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, ["a", "more", "open", "b"]);
        assert_eq!(paragraphs[1].numbering, None);
        assert_eq!(
            paragraphs[1].style.as_deref(),
            Some("ListParagraphContinue")
        );
        assert_eq!(paragraphs[3].style.as_deref(), Some("ListParagraph"));
        assert_eq!(paragraphs[3].numbering, paragraphs[0].numbering);
    }
}
