//! A simple fuzz run, with no tooling beyond the tests: every `.adoc` fixture, cut off at each line
//! and at a spread of character positions, and with small deterministic mutations (a character
//! deleted, doubled, or replaced with a markup character), is parsed and rendered through both
//! HTML backends. Each input runs on its own thread with a timeout, so a hang is reported rather
//! than hanging the run. A parse error is fine (the input is often broken); a panic or a hang is a
//! bug, and each one found should become a test of its own.
//!
//! The quick run (in the default test run) cuts the fixtures off at each line; the full run, with
//! the mutations, is `cargo test --all-features --test fuzz_tests -- --ignored`.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use asciidocr::backends::htmls::{render_asciidoctor_html, render_htmlbook};
use asciidocr::parser::Parser;
use asciidocr::scanner::Scanner;

/// How long one input may take before it counts as a hang. The fixtures are small: each parses
/// and renders in a few milliseconds, even in a debug build, so this is hundreds of times longer
/// than any of them needs
const TIMEOUT: Duration = Duration::from_secs(2);

/// A thread that hangs can't be stopped, and keeps a core busy, which slows (and eventually
/// spoils) the rest of the run; so the run stops after this many hangs
const MAX_HANGS: usize = 5;

/// Characters that start or end markup, for the mutations
const MARKUP: &[char] = &[
    '*', '_', '`', '#', '^', '~', '+', '[', ']', '<', '>', ':', ';', '|', '=', '.', '-', '/', '\n',
    ' ', '{', '}', '"', '\'',
];

fn fixtures() -> Vec<PathBuf> {
    fn collect(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("Unable to read tests/data") {
            let path = entry.expect("Unable to read a directory entry").path();
            if path.is_dir() {
                collect(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "adoc") {
                found.push(path);
            }
        }
    }
    let mut found = vec![];
    collect(Path::new("tests/data"), &mut found);
    found.sort();
    found
}

/// The fixture cut off after each of its lines
fn line_prefixes(source: &str) -> Vec<String> {
    source
        .match_indices('\n')
        .map(|(idx, _)| source[..idx].to_string())
        .collect()
}

/// The fixture cut off at up to `count` character positions spread over it
fn char_prefixes(source: &str, count: usize) -> Vec<String> {
    let boundaries: Vec<usize> = source.char_indices().map(|(idx, _)| idx).collect();
    let step = (boundaries.len() / count).max(1);
    boundaries
        .iter()
        .step_by(step)
        .map(|&idx| source[..idx].to_string())
        .collect()
}

/// `count` deterministic mutations of the fixture: at positions spread over it, a character is
/// deleted, doubled, or replaced with a markup character
fn mutations(source: &str, count: usize) -> Vec<String> {
    let chars: Vec<char> = source.chars().collect();
    if chars.is_empty() {
        return vec![];
    }
    // a small linear congruential generator, so the run is the same every time
    let mut state: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as usize
    };
    (0..count)
        .map(|_| {
            let mut mutated = chars.clone();
            let position = next() % mutated.len();
            match next() % 3 {
                0 => {
                    mutated.remove(position);
                }
                1 => mutated.insert(position, mutated[position]),
                _ => mutated[position] = MARKUP[next() % MARKUP.len()],
            }
            mutated.into_iter().collect()
        })
        .collect()
}

/// Parses and renders one input on its own thread: `Err` describes a panic or a hang
fn check(origin: &Path, input: String) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel();
    let origin = origin.to_path_buf();
    let handle = thread::spawn(move || {
        if let Ok(graph) = Parser::new_no_target_resolution(origin).parse(Scanner::new(&input)) {
            // rendering may fail on a broken document; it mustn't panic
            let _ = render_htmlbook(&graph);
            let _ = render_asciidoctor_html(&graph);
        }
        let _ = sender.send(());
    });
    match receiver.recv_timeout(TIMEOUT) {
        Ok(()) => handle
            .join()
            .map_err(|_| "panicked (after finishing?)".to_string()),
        Err(mpsc::RecvTimeoutError::Timeout) => Err("hung".to_string()),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err("panicked".to_string()),
    }
}

/// Runs every input the `variants` of each fixture give, and fails with a list of the ones that
/// panicked or hung
fn fuzz(variants: impl Fn(&str) -> Vec<String>) {
    // panics are expected to be reported below, not printed as they happen
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut failures = vec![];
    let mut hangs = 0;
    let mut count = 0;
    'fixtures: for path in fixtures() {
        let source = std::fs::read_to_string(&path).expect("Unable to read a fixture");
        for input in variants(&source) {
            count += 1;
            if let Err(problem) = check(&path, input.clone()) {
                let failure = format!("{} {problem}:\n{input:?}", path.display());
                // (shown as they happen with --nocapture, in case the run is stopped)
                eprintln!("{failure}\n");
                failures.push(failure);
                if problem == "hung" {
                    hangs += 1;
                    if hangs == MAX_HANGS {
                        failures.push(format!("(stopped after {MAX_HANGS} hangs)"));
                        break 'fixtures;
                    }
                }
            }
        }
    }
    std::panic::set_hook(default_hook);
    assert!(
        failures.is_empty(),
        "{} of {count} inputs failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[test]
fn test_fixtures_cut_off_at_each_line() {
    fuzz(line_prefixes);
}

#[test]
#[ignore = "slow: the full fuzz run, with --ignored"]
fn test_fixtures_cut_off_and_mutated() {
    fuzz(|source| {
        let mut inputs = char_prefixes(source, 40);
        inputs.extend(mutations(source, 60));
        inputs
    });
}
