pub mod common;
use std::sync::{Mutex, MutexGuard, OnceLock};

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
