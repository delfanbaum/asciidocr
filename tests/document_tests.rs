//use std::fs;
//
//use asciidocr::{parser::Parser, scanner::Scanner};
//
//pub mod common;
//
//
//#[test]
/////Smoke test for "have we covered enough" -- will be added to as we go along.
//fn test_targeted_coverage() {
//    let adoc_fn = "tests/data/documents/minimum.adoc";
//    let _ = Parser::new().parse(Scanner::new(
//        &fs::read_to_string(adoc_fn).expect("Unable to find adoc")
//    ));
//}
