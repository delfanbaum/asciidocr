//! Tests that run the `asciidocr` binary itself, to check the pieces the library tests do not
//! reach: reading from stdin, the `--backend` and `--embedded` options, and exit codes.

use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

use rstest::rstest;

/// Runs `asciidocr` with the given arguments, feeding it `input` on stdin
fn run_asciidocr(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_asciidocr"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Unable to start asciidocr");
    child
        .stdin
        .take()
        .expect("No stdin")
        .write_all(input.as_bytes())
        .expect("Unable to write to stdin");
    child.wait_with_output().expect("asciidocr did not finish")
}

const DOCUMENT: &str = "= A Title\n\nA paragraph with *strong* text.\n";

#[rstest]
#[case::asciidoctor_full(
    &["-b", "asciidoctor", "-o", "-", "-"],
    &["<title>A Title</title>", "<div class=\"paragraph\">", "<strong>strong</strong>", "<div id=\"footer\">"],
    &[]
)]
#[case::asciidoctor_embedded(
    &["-b", "asciidoctor", "-e", "-o", "-", "-"],
    &["<div class=\"paragraph\">", "<strong>strong</strong>"],
    &["<html", "<div id=\"footer\">", "<title>"]
)]
#[case::htmlbook_full(
    &["-b", "htmlbook", "-o", "-", "-"],
    &["<title>A Title</title>", "<p>A paragraph with <strong>strong</strong> text.</p>"],
    &[]
)]
#[case::htmlbook_embedded(
    &["-b", "htmlbook", "-e", "-o", "-", "-"],
    &["<p>A paragraph with <strong>strong</strong> text.</p>"],
    &["<html", "<title>"]
)]
fn test_html_backends(
    #[case] args: &[&str],
    #[case] expected_in_output: &[&str],
    #[case] not_expected_in_output: &[&str],
) {
    let output = run_asciidocr(args, DOCUMENT);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("Output is not UTF-8");
    for expected in expected_in_output {
        assert!(
            stdout.contains(expected),
            "Missing {expected:?} in:\n{stdout}"
        );
    }
    for unexpected in not_expected_in_output {
        assert!(
            !stdout.contains(unexpected),
            "Found {unexpected:?} in:\n{stdout}"
        );
    }
}

#[test]
fn test_unknown_backend_fails() {
    let output = run_asciidocr(&["-b", "nonsense", "-o", "-", "-"], DOCUMENT);
    assert!(!output.status.success());
}
