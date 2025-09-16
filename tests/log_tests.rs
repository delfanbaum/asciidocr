pub mod common;
use common::assert_parsed_doc_matches_expected_asg;
use logtest::Logger;

#[test]
fn test_missing_references_warns() {
    let logger = Logger::start();
    let fn_pattern = "documents/references-missing";
    let adoc_fn = format!("{}.adoc", fn_pattern);
    let asg_json_fn = format!("{}.json", fn_pattern);
    assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn);
    let log_str = logger.last().unwrap().args().to_owned();
    assert!(log_str.contains("Unable to find xref:"));
}
