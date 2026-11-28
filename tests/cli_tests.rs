//! Tests that run the `asciidocr` binary itself, to check the pieces the library tests do not
//! reach: reading from files and stdin, where the output goes, the command-line options, and exit
//! codes.

use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

use rstest::rstest;
use tempfile::{TempDir, tempdir};

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

/// Runs `asciidocr` with the given arguments in `dir`, with nothing on stdin
fn run_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_asciidocr"))
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .expect("Unable to run asciidocr")
}

/// A temporary directory holding one file
fn dir_with(name: &str, contents: &str) -> TempDir {
    let dir = tempdir().expect("Unable to make a temporary directory");
    fs::write(dir.path().join(name), contents).expect("Unable to write the input file");
    dir
}

/// The names of the files in a directory, in order
fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("Unable to read the directory")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Asserts that a run failed in the ordinary way (not with a panic), saying why on stderr
fn assert_fails_with(output: &Output, message: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains(message),
        "Missing {message:?} in:\n{stderr}"
    );
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

/// `-d` shows debug output whether or not `-v` is also given, since debug is the more detailed
/// of the two
#[rstest]
#[case::debug(&["-d", "-o", "-", "-"], true)]
#[case::verbose_and_debug(&["-v", "-d", "-o", "-", "-"], true)]
#[case::verbose(&["-v", "-o", "-", "-"], false)]
fn test_debug_flag_shows_debug_output(#[case] args: &[&str], #[case] shows_debug: bool) {
    let output = run_asciidocr(args, DOCUMENT);
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.contains("DEBUG"), shows_debug, "{stderr}");
}

/// Log messages go to stderr, so they never end up in a document written to stdout
#[test]
fn test_logging_stays_out_of_stdout() {
    let output = run_asciidocr(&["-d", "-o", "-", "-"], "image::missing.png[]\n");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stdout.contains("DEBUG") && !stdout.contains("WARN"),
        "{stdout}"
    );
    assert!(stderr.contains("WARN"), "{stderr}");
}

/// Without `-o`, the output goes next to the input, with the backend's extension
#[rstest]
#[case::default_backend(&["doc.adoc"], "doc.html")]
#[case::asciidoctor(&["-b", "asciidoctor", "doc.adoc"], "doc.html")]
#[cfg_attr(feature = "json", case::json(&["-b", "json", "doc.adoc"], "doc.json"))]
#[cfg_attr(feature = "docx", case::docx(&["-b", "docx", "doc.adoc"], "doc.docx"))]
fn test_default_output_file(#[case] args: &[&str], #[case] written: &str) {
    let dir = dir_with("doc.adoc", DOCUMENT);
    let output = run_in(dir.path(), args);
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let mut expected = vec!["doc.adoc", written];
    expected.sort();
    assert_eq!(file_names(dir.path()), expected);
    assert!(fs::metadata(dir.path().join(written)).unwrap().len() > 0);
}

#[test]
fn test_named_output_file() {
    let dir = dir_with("doc.adoc", DOCUMENT);
    let output = run_in(dir.path(), &["-o", "other.htm", "doc.adoc"]);
    assert!(output.status.success());
    assert_eq!(file_names(dir.path()), ["doc.adoc", "other.htm"]);
    let written = fs::read_to_string(dir.path().join("other.htm")).unwrap();
    assert!(written.contains("<strong>strong</strong>"), "{written}");
}

#[cfg(feature = "json")]
#[test]
fn test_json_to_stdout() {
    let output = run_asciidocr(&["-b", "json", "-o", "-", "-"], DOCUMENT);
    assert!(output.status.success());
    let graph: serde_json::Value = serde_json::from_slice(&output.stdout).expect("Not JSON");
    assert_eq!(graph["name"], "document");
}

#[cfg(feature = "docx")]
#[rstest]
#[case::to_stdout(&["-b", "docx", "-o", "-", "doc.adoc"], "can't send docx backend to stdout")]
#[case::unresolved_targets(
    &["-b", "docx", "-x", "doc.adoc"],
    "does not support parsing without target resolution"
)]
fn test_docx_refusals(#[case] args: &[&str], #[case] message: &str) {
    let dir = dir_with("doc.adoc", DOCUMENT);
    assert_fails_with(&run_in(dir.path(), args), message);
    assert_eq!(file_names(dir.path()), ["doc.adoc"]);
}

#[test]
fn test_missing_input_file_fails() {
    let dir = tempdir().unwrap();
    assert_fails_with(
        &run_in(dir.path(), &["missing.adoc"]),
        "Unable to read file",
    );
    assert!(file_names(dir.path()).is_empty());
}

#[rstest]
#[case::adoc("doc.adoc")]
#[case::asciidoc("doc.asciidoc")]
#[case::text("doc.txt")]
#[case::upper_case("DOC.ADOC")]
fn test_input_extensions(#[case] input: &str) {
    let dir = dir_with(input, DOCUMENT);
    let output = run_in(dir.path(), &["-o", "-", input]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A file that isn't named as AsciiDoc is refused, whatever else is asked for
#[rstest]
#[case::html("page.html", &["page.html"])]
#[case::markdown("notes.md", &["-o", "out.html", "notes.md"])]
#[case::no_extension("README", &["README"])]
#[case::counting("page.html", &["-c", "page.html"])]
fn test_other_input_extensions_fail(#[case] input: &str, #[case] args: &[&str]) {
    let dir = dir_with(input, DOCUMENT);
    assert_fails_with(&run_in(dir.path(), args), ".adoc, .asciidoc or .txt");
    assert_eq!(file_names(dir.path()), [input]);
    assert_eq!(
        fs::read_to_string(dir.path().join(input)).unwrap(),
        DOCUMENT
    );
}

/// The input is never replaced by the output, however the output is named
#[rstest]
#[case::same_name("doc.adoc", &["-o", "doc.adoc", "doc.adoc"])]
#[case::another_path_to_it("doc.adoc", &["-o", "./doc.adoc", "doc.adoc"])]
fn test_output_does_not_overwrite_input(#[case] input: &str, #[case] args: &[&str]) {
    let dir = dir_with(input, DOCUMENT);
    assert_fails_with(&run_in(dir.path(), args), "would overwrite the input file");
    assert_eq!(
        fs::read_to_string(dir.path().join(input)).unwrap(),
        DOCUMENT
    );
}

#[rstest]
#[case::html(&["-o", "no-such-dir/out.html", "doc.adoc"])]
#[cfg_attr(
    feature = "docx",
    case::docx(&["-b", "docx", "-o", "no-such-dir/out.docx", "doc.adoc"])
)]
fn test_unwritable_output_fails(#[case] args: &[&str]) {
    let dir = dir_with("doc.adoc", DOCUMENT);
    let output = run_in(dir.path(), args);
    assert_fails_with(&output, "Error converting document");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
}

/// `-c` prints the word count and nothing else, and writes no file
#[rstest]
#[case::several("one two three\n", &["-c", "doc.adoc"], "3 words in doc.adoc\n")]
#[case::one("one\n", &["-c", "doc.adoc"], "1 word in doc.adoc\n")]
#[case::none("", &["-c", "doc.adoc"], "0 words in doc.adoc\n")]
#[case::with_output_options(
    "one two\n",
    &["-c", "-b", "asciidoctor", "-o", "-", "doc.adoc"],
    "2 words in doc.adoc\n"
)]
fn test_count_words(#[case] document: &str, #[case] args: &[&str], #[case] expected: &str) {
    let dir = dir_with("doc.adoc", document);
    let output = run_in(dir.path(), args);
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
    assert_eq!(file_names(dir.path()), ["doc.adoc"]);
}
