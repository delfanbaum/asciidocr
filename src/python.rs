#![cfg(feature = "python")]

use crate::backends::htmls::render_htmlbook;
use crate::parser;
use crate::scanner;
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use std::path::PathBuf;

/// parses an asciidoc string to HTML (using the default HTML build)
#[pyfunction]
fn parse_to_html(adoc_str: &str) -> PyResult<String> {
    let graph = parser::Parser::new(PathBuf::from("-")).parse(scanner::Scanner::new(adoc_str));
    match render_htmlbook(&graph) {
        Ok(html) => Ok(html),
        Err(_) => Err(PyRuntimeError::new_err(
            "Error converting asciidoc string to HTML",
        )),
    }
}

/// parses an asciidoc string to a Json string, which can then be converted using Python's
/// json package from standard library (or some other function(s)) into a dict
#[pyfunction]
fn parse_to_json_str(adoc_str: &str) -> PyResult<String> {
    let graph = parser::Parser::new(PathBuf::from("-")).parse(scanner::Scanner::new(adoc_str));
    match serde_json::to_string_pretty(&graph) {
        Ok(json_str) => Ok(json_str),
        Err(_) => Err(PyRuntimeError::new_err(
            "Error converting asciidoc string to JSON",
        )),
    }
}

#[pymodule]
fn asciidocr(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let _ = m.add_function(wrap_pyfunction!(parse_to_html, m)?);
    let _ = m.add_function(wrap_pyfunction!(parse_to_json_str, m)?);
    Ok(())
}
