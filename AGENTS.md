# AGENTS.md

Guidance for AI coding agents working in `asciidocr`, a pure-Rust CLI and library that parses
Asciidoc into an Abstract Syntax Graph (ASG) and renders it through backends.

## Architecture

The pipeline is `Scanner` → `Parser` → `Asg` → backend.

- `src/scanner/`: turns source text into `Token`s (`tokens.rs` holds `Token`/`TokenType`).
  The scanner is an iterator of `Result<Token, ScannerError>`.
- `src/parser.rs`: consumes tokens and builds the `Asg`. It uses stacks for blocks and inlines,
  and it resolves includes, attributes and targets. Lists (description lists too) stay off the
  block stack while they are open, in `src/parser/list_context.rs`; its module docs explain the
  scopes.
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
cargo clippy --all-features --all-targets
```

- Always pass `--all-features`. The `docx` and `json` code and tests only compile with their
  features enabled, so a plain `cargo test` skips them.
- The author uses `bacon` (`bacon.toml`). Its default job is `test-features`, and `c` runs
  clippy with all features.
- Clippy (with `--all-targets`, so tests are linted too) and `cargo fmt --check` are both clean. Keep them that way: don't add warnings, and
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
- **Integration tests** live in `tests/*_tests.rs`, named by area: `block_tests`, `list_tests`,
  `inline_tests`, `document_tests`, `htmlbook_tests`, `asciidoctor_html_tests`, `docx_tests`, `log_tests`,
  `offset_tests`, and `cli_tests` (which runs the `asciidocr` binary itself). Each one starts
  with `pub mod common;` (except `cli_tests`) and uses the shared helpers in `tests/common/mod.rs`:
  - `assert_parsed_doc_matches_expected_asg(adoc_fn, json_fn)`
  - `assert_parsed_doc_matches_expected_asg_from_str(adoc, json)`
  - `assert_rendered_htmlbook_matches_expected(adoc_fn, html_fn)` and its `_embedded_` sibling.
    They ignore whitespace.
  - `assert_rendered_asciidoctor_html_matches_expected(adoc_fn, html_fn)` (a full document; it
    ignores whitespace and the "Last updated" time) and
    `assert_rendered_asciidoctor_embedded_matches_expected`
  - `parsed_text(adoc)`, `top_level_block_names(adoc)` and `block_tree(adoc)` for tests that
    only care about a document's text or how its blocks nest, without hand-writing JSON with
    locations
  - `block_outline(adoc)` (and `block_outline_with_timeout`, for inputs that could hang): like
    `block_tree`, but it descends into lists and shows each block's text, e.g.
    `list(* "one"(paragraph "more"), * "two")`. Use it for list tests
  - `rendered_docx_part(adoc, part)`, `rendered_docx_runs(adoc)` and
    `assert_rendered_docx_runs_match_expected(adoc_fn, json_fn)` for docx: the runs of each
    paragraph (text, bold, italic, ...) are compared against a `.json` fixture in `tests/data/docx/`

  Add a helper to `common` instead of repeating parse/render boilerplate.
- **Fixtures** live in `tests/data/<area>/`, as an `.adoc` file and its expected output under the
  same kebab-case base name:
  - `blocks/`, `inlines/`, `documents/`: `name.adoc` + `name.json` (expected ASG)
  - `htmlbook/`: `name.adoc` + `name.html`
  - `asciidoctor-html/`: `name.adoc` + `name.html` (a full document), and
    `asciidoctor-html/embedded/`: the same, but the `.html` is embedded output
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
- **Whitespace**: the HTML comparison helpers ignore whitespace, so they cannot see bugs inside
  `<pre>` or verse blocks. When whitespace matters, add an exact test (render to a string with
  `render_htmlbook_embedded_from_str`/`render_asciidoctor_embedded_from_str` and `assert!(html.contains(..))`).
- **Logging** (`tests/log_tests.rs`): `Logger::start` can only be called once per process, so the
  tests share one `logtest` logger behind a mutex (see the `logger()` helper); anything logged by
  earlier tests is discarded when a test takes the lock.
- **Hangs**: a test that could loop forever should parse on its own thread and use
  `recv_timeout`, so that a regression fails the test instead of hanging the run (see
  `test_unclosed_include_is_text`).
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
  or scratch files such as `scratch.html`.

## Current state (update as this changes)

- The migration from Tera to minijinja is finished: both HTML backends (HTMLBook and
  Asciidoctor-style) render with minijinja templates under `templates/<backend>/*.jinja`, each
  with its own block and inline templates.
- The Asciidoctor-style templates copy Asciidoctor's markup, and Asciidoctor itself (2.0.17) is the
  oracle: fixtures in `tests/data/asciidoctor-html/embedded/` are `asciidoctor -s -o - name.adoc`
  output (embedded mode, i.e., document contents only), and the five full-document fixtures in
  `tests/data/asciidoctor-html/` cover the shell (header, content, footer). Numbering
  ("Figure 1."), section ids, footnotes and image alt text need per-render state or helpers, so
  they are template functions registered in `src/backends/htmls.rs` (`next_number`, `section_id`,
  `register_footnote`/`footnotes`, `image_alt`). Those tests ignore whitespace, so whitespace
  that matters (inside `<pre>`) has its own exact-match test.
- minijinja environments in `src/backends/htmls.rs` have autoescape turned off, because the parser
  makes character references (`&#8217;`, `&amp;`) of replacements and of references the author
  wrote, and those must not be escaped again. Templates escape the rest themselves, with the
  `escape_text` filter (text: `&`, `<`, `>`) on `text` literals and the `escape_attr` filter
  (also `"`) on every attribute value. Passthrough content (`pass` blocks, `raw` literals) is
  output as written. Don't add `| safe`.
- Templates report recoverable problems in the document through the `warn(...)` template function,
  with a `"Line {n}: …"` message (e.g., the HTMLBook list-title warning); tests assert on these
  (`tests/log_tests.rs`), so don't reword them casually.
- **Titles** (decided 2026-11-25, checked against Asciidoctor 2.0.17):

  | Block | HTMLBook | Asciidoctor-style |
  | --- | --- | --- |
  | listing, literal, verse | wrapped in `<div data-type="example">` with an `<h5>` | `<div class="title">` |
  | example, sidebar | `<h5>` | `<div class="title">`; examples are numbered ("Example 1.") |
  | admonition (both forms) | `<h1>` | `<div class="title">` in the content cell |
  | table | `<caption>` | `<caption class="title">`, numbered ("Table 1.") |
  | image | `<figcaption>` | `<div class="title">`, numbered ("Figure 1.") |
  | list, description list | kept in the ASG, not rendered; a `warn!` is logged | rendered |
  | paragraph | not supported: dropped with a warning (the paragraph is kept) | same |

  The DOCX backend renders none of the list titles.
- HTMLBook block macros live in one file (`templates/htmlbook/block.html.jinja`) so they can
  recurse through `render_block`. minijinja filters take positional args (`join(" ")`, not
  `join(sep=" ")`), and attribute access on a missing `metadata` errors, so guard it first.
