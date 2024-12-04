//! Fast and (eventually) compliant Asciidoc parsing!
//!
//! (For information about Asciidoc, see <https://asciidoc.org/>)
//!
//! This crate provides a CLI tool (`asciidocr`) for working with/building asciidoc files, a way to
//! interface with the official Technology Compatibility Kit adapter (see
//! <https://gitlab.eclipse.org/eclipse/asciidoc-lang/asciidoc-tck>) via the `json` backend, and
//! library access to the parser, scanner, backends, and Abstract Syntax Graph elements.
//!
//! NOTE: This crate is still in progress and nothing, including library elements, should be
//! considered stable. If something disappears that you're interested in, please open an [`issue`].
//!
//! While eventually the goal is to support the vast majority of the language features, many are
//! not yet implemented. Notable misses include:
//!
//! - Some Asciidoctor document attributes (e.g., `:toc:`, `:icons:`, etc.)
//! - Indented source blocks
//! - Offsets
//! - Tagged regions
//! - Conditionals (`ifdef`, `ifndef`, `ifeval`)
//!
//! It's also important to note that though we have targeted (and are passing) all of the
//! compatibility tests included in the TCK, there have been areas where we've deviated from the
//! published schema, esp. in cases where it's not obvious what's to be done.
//!
//! Current backends (parse targets) includes:
//!
//! - [`HTMLBook`]: fairly good support; can be used as a relatively "unadorned" HTML generator
//! - Docx (behind `docx` feature):  experimental and still very much in-progress; but good enough
//!   for "simple" documents without tables, images, etc.
//!
//! [`issue`]: https://github.com/delfanbaum/asciidocr/issues
//! [`HTMLBook`]: https://oreillymedia.github.io/HTMLBook/

pub mod backends;
pub mod graph;
pub mod parser;
pub mod scanner;

#[cfg(feature = "docx")]
mod python;
