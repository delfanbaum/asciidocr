pub mod common;
use std::sync::{Mutex, MutexGuard, OnceLock};

use asciidocr::{parser::Parser, scanner::Scanner};
use common::{assert_parsed_doc_matches_expected_asg, assert_rendered_htmlbook_matches_expected};
use logtest::Logger;

/// `Logger::start` can only be called once per process, so the tests share one logger, and the
/// lock keeps them from running at the same time. Anything logged by earlier tests is discarded.
static LOGGER: OnceLock<Mutex<Logger>> = OnceLock::new();

fn logger() -> MutexGuard<'static, Logger> {
    let mut logger = LOGGER
        .get_or_init(|| Mutex::new(Logger::start()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    while logger.pop().is_some() {}
    logger
}

#[test]
fn test_missing_references_warns() {
    let mut logger = logger();
    let fn_pattern = "documents/references-missing";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn);
    let log_str = logger.by_ref().last().unwrap().args().to_owned();
    assert!(log_str.contains("Unable to find xref:"));
}

/// An xref to a generated section id finds it
#[test]
fn test_xref_to_generated_id_does_not_warn() {
    let mut logger = logger();
    common::parsed_asg("== My Section\n\nSee <<_my_section>>.\n");
    let mut warnings = vec![];
    while let Some(record) = logger.pop() {
        warnings.push(record.args().to_owned());
    }
    assert!(
        !warnings.iter().any(|w| w.contains("Unable to find xref")),
        "{warnings:?}"
    );
}

#[test]
fn test_title_on_paragraph_warns() {
    let mut logger = logger();
    assert_rendered_htmlbook_matches_expected(
        "htmlbook/paragraph-title-dropped.adoc",
        "htmlbook/paragraph-title-dropped.html",
    );
    let mut warnings = vec![];
    while let Some(record) = logger.pop() {
        warnings.push(record.args().to_owned());
    }
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("Line 3") && w.contains("Titles on paragraphs are not supported")),
        "expected a paragraph title warning, got: {warnings:?}"
    );
}

#[test]
fn test_title_on_list_warns() {
    let mut logger = logger();
    assert_rendered_htmlbook_matches_expected(
        "htmlbook/list-title-not-rendered.adoc",
        "htmlbook/list-title-not-rendered.html",
    );
    let mut warnings = vec![];
    while let Some(record) = logger.pop() {
        warnings.push(record.args().to_owned());
    }
    assert!(
        warnings.iter().any(|w| w.contains("Line 2")
            && w.contains("Titles on lists are not rendered by the HTMLBook backend yet")),
        "expected a list title warning, got: {warnings:?}"
    );
}

#[test]
fn test_missing_block_image_target_warns_and_renders() {
    let mut logger = logger();
    let adoc = ".Figure Title\nimage::does-not-exist.png[Alt]\n\nAfter.\n";
    // unlike the shared helpers, this parser resolves targets
    let asg = Parser::new(std::env::current_dir().unwrap())
        .parse(Scanner::new(adoc))
        .expect("a missing image target should not abort parsing");
    let json = serde_json::to_string(&asg).unwrap();
    assert!(
        json.contains("does-not-exist.png"),
        "image block missing: {json}"
    );
    assert!(
        json.contains("After."),
        "following paragraph missing: {json}"
    );
    let mut warnings = vec![];
    while let Some(record) = logger.pop() {
        warnings.push(record.args().to_owned());
    }
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("Line 2") && w.contains("Unable to resolve target")),
        "expected a missing target warning, got: {warnings:?}"
    );
}

#[test]
fn test_missing_inline_image_target_warns_and_renders() {
    let mut logger = logger();
    let adoc = "Some text image:does-not-exist.png[Alt] more text.\n";
    // unlike the shared helpers, this parser resolves targets
    let asg = Parser::new(std::env::current_dir().unwrap())
        .parse(Scanner::new(adoc))
        .expect("a missing image target should not abort parsing");
    let json = serde_json::to_string(&asg).unwrap();
    assert!(
        json.contains("does-not-exist.png"),
        "inline image missing: {json}"
    );
    let mut warnings = vec![];
    while let Some(record) = logger.pop() {
        warnings.push(record.args().to_owned());
    }
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("Line 1") && w.contains("Unable to resolve target")),
        "expected a missing target warning, got: {warnings:?}"
    );
}

/// The docx backend can't render footnotes yet, so it warns once per footnote (not once per run
/// of text inside it), through the logger rather than straight to stderr
#[cfg(feature = "docx")]
#[test]
fn test_docx_footnote_warns_once() {
    let mut logger = logger();
    common::rendered_docx_runs("Some text.\n\nA footnote:[With *bold* and plain text.] here.\n");
    let mut warnings = vec![];
    while let Some(record) = logger.pop() {
        warnings.push(record.args().to_owned());
    }
    let footnote_warnings: Vec<_> = warnings
        .iter()
        .filter(|w| w.contains("Footnotes are not well supported"))
        .collect();
    assert_eq!(footnote_warnings.len(), 1, "{warnings:?}");
    assert!(footnote_warnings[0].contains("Line 3"), "{warnings:?}");
}
