# AGENTS.md

Guidance for AI coding agents working in `asciidocr`, a pure-Rust CLI and library that parses
Asciidoc into an Abstract Syntax Graph (ASG) and renders it through backends.

## Architecture

The pipeline is `Scanner` → `Parser` → `Asg` → backend.

- `src/scanner/`: turns source text into `Token`s (`tokens.rs` holds `Token`/`TokenType`).
  The scanner is an iterator of `Result<Token, ScannerError>`.
- `src/parser.rs`: consumes tokens and builds the `Asg`. It uses stacks for blocks and inlines,
  and it resolves includes, attributes and targets.
- `src/graph/`: the ASG types (`asg`, `blocks`, `inlines`, `lists`, `metadata`, `nodes`,
  `substitutions`). They roughly follow the official Asciidoc ASG schema and are all
  `serde`-serializable. That serialization is how the JSON/TCK output and the templates work.
- `src/backends/`: `htmls.rs` (HTMLBook, rendered with minijinja) and `docx/` (behind the `docx`
  feature). The `Backends` enum in `backends/mod.rs` is also the CLI's `--backend` value enum.
- `src/errors.rs`: every error type, defined with `thiserror`.
- `src/bin/asciidocr/`: the CLI (`clap` derive in `cli.rs`).
- `templates/`: HTML templates, embedded at compile time with `include_str!`. Changing a
  template requires a rebuild.

## Build, lint, test

CI (`.github/workflows/`) runs these, so run them before you call a change done:

```console
cargo build --all-features
cargo test --all-features
```

Also run:

```console
cargo fmt                      # default rustfmt settings; there is no rustfmt.toml
cargo clippy --all-features
```

- Always pass `--all-features`. The `docx` and `json` code and tests only compile with their
  features enabled, so a plain `cargo test` skips them.
- The author uses `bacon` (`bacon.toml`). Its default job is `test-features`, and `c` runs
  clippy with all features.
- Clippy and `cargo fmt --check` are both clean. Keep them that way: don't add warnings, and
  put any lint-only or format-only change in its own commit.

## Code conventions

- Rust 2024 edition. Follow standard Rust naming: `snake_case` for functions and modules,
  `CamelCase` for types and enum variants.
- ASG types use descriptive compound names, such as `LeafBlock`, `LeafBlockName`,
  `LeafBlockForm`, `InlineSpan`, `InlineSpanVariant`, `InlineSpanForm` and `InlineLiteralName`.
  A node's kind, form and variant go in companion enums, not strings. Follow this pattern for new
  node types.
- Constructors are `new(...)` plus named alternatives such as `new_default`,
  `new_text_from_token` and `new_no_target_resolution`.
- Errors: add a variant to the appropriate enum in `src/errors.rs` (`ScannerError`,
  `ParserError`, `BlockError`, `AsgError`, `ConversionError`, ...). Use `#[error(...)]` messages,
  and start them with `"Line {0}: ..."` when a line number is available. Wrap errors with
  `#[from]`/`#[error(transparent)]`, and return `Result` rather than panicking in library code.
  `anyhow` is used only at the binary/CLI level.
- Logging: use the `log` macros (`warn!`, `info!`, `error!`). Use `warn!` for recoverable problems
  in documents, such as missing attributes, unresolved xrefs and unreadable includes. Tests
  assert on these messages (`tests/log_tests.rs`), so don't reword them casually.
- Feature-gated code uses `#[cfg(feature = "docx")]` / `#[cfg(feature = "json")]` on the module,
  the enum variant and the error variant alike.
- Docs: every module gets a `//!` doc header, and public items get `///` docs. Write comments in
  plain prose that explain *why*. Note known gaps in `LIMITATIONS.adoc` and, where relevant, in
  the crate docs in `src/lib.rs`.

## Testing conventions

- **Unit tests** live inline at the bottom of the file, in `#[cfg(test)] mod tests { use super::*; ... }`.
  Use small local helpers (for example `scan_and_assert_eq`, `newline_token_at`).
- **Integration tests** live in `tests/*_tests.rs`, named by area: `block_tests`, `inline_tests`,
  `document_tests`, `htmlbook_tests`, `docx_tests`, `log_tests`, `offset_tests`. Each one starts
  with `pub mod common;` and uses the shared assertion helpers in `tests/common/mod.rs`:
  - `assert_parsed_doc_matches_expected_asg(adoc_fn, json_fn)`
  - `assert_parsed_doc_matches_expected_asg_from_str(adoc, json)`
  - `assert_rendered_htmlbook_matches_expected(adoc_fn, html_fn)`. It ignores whitespace.

  Add a helper to `common` instead of repeating parse/render boilerplate.
- **Fixtures** live in `tests/data/<area>/`, as an `.adoc` file and its expected output under the
  same kebab-case base name:
  - `blocks/`, `inlines/`, `documents/`: `name.adoc` + `name.json` (expected ASG)
  - `htmlbook/`: `name.adoc` + `name.html`
  - `asciidoctor-html/`: `name.adoc` + `name.html`
- Use `rstest` for parameterized cases, and name each case after its fixture:

  ```rust
  #[rstest]
  #[case::para_single_line("blocks/para-single-line")]
  fn test_paragraphs(#[case] fn_pattern: &str) {
      let adoc_fn = format!("{}.adoc", fn_pattern);
      let asg_json_fn = format!("{}.json", fn_pattern);
      assert_parsed_doc_matches_expected_asg(&adoc_fn, &asg_json_fn)
  }
  ```

  Single-fixture tests use a plain `#[test]` with a descriptive `snake_case` name, as in
  `htmlbook_tests.rs`.
- Tests that need real files use `tempfile` (and `image` to generate test images). Parse with
  `Parser::new_no_target_resolution` unless the test is about target resolution.
- Bug fixes start with a failing test and fixture that reproduce the bug.
- Never edit an expected `.json`/`.html` fixture just to make a test pass. Fix the code, or
  explain why the expected output was wrong.

## Git and workflow

- Commit messages are short, imperative and sentence-case, with no prefix or scope. Examples:
  "Handle parsing TokenType::Hyperlink", "Put json backend behind feature flag",
  "Solve for charrefs appearing inside span elements".
- Format-only and lint-only changes get their own commits ("Cargo fmt", "Clippy").
- Version bumps are separate commits ("Bump version").
- Feature and bug branches merge into `main` via PR.
- Don't commit anything from `notes/` or `local-tests/` (both are gitignored, as are `*.docx`),
  or scratch files such as `scratch.tera`.

## Current state (update as this changes)

- The project is migrating from Tera to minijinja (`templates/*.jinja`). The HTMLBook templates
  are fully ported and `htmlbook_tests` pass. The asciidoctor backend's templates are still
  `.tera`. Its tests are parked in `tests/asciidoctor_html_tests.rs.hold`, and the matching
  helper in `tests/common/mod.rs` is commented out.
- minijinja environments in `src/backends/htmls.rs` have autoescape turned off, because the parser
  already entity-encodes text. Don't add `| safe` or re-escape in templates.
- HTMLBook block macros live in one file (`templates/htmlbook/block.html.jinja`) so they can
  recurse through `render_block`. minijinja filters take positional args (`join(" ")`, not
  `join(sep=" ")`), and attribute access on a missing `metadata` errors, so guard it first.
