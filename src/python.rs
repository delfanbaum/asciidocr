#![cfg(feature="python")]

use std::path::PathBuf;
use crate::scanner;
use crate::parser;
use crate::backends::htmls::render_htmlbook;
use pyo3::{exceptions::PyRuntimeError, prelude::*};

/// parses a string using the specified backend
#[pyfunction]
fn parse_to_html(adoc_str: &str) -> PyResult<String> {
    let graph = parser::Parser::new(PathBuf::from("-")).parse(scanner::Scanner::new(adoc_str));
    match render_htmlbook(&graph) {
        Ok(html) => Ok(html),
        Err(_) => Err(PyRuntimeError::new_err("Error converting asciidoc string")),
    }
}

#[pymodule]
fn asciidocr(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_to_html, m)?)
}
