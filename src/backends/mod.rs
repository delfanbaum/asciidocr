//! Backends serve as the "targets" for the parser, which itself only produces an Asg, or
//! Abstract Syntax Graph. Currently the backends include:
//! 
//! - HTMLBook (fairly good support; can be used as a relatively "unadorned" HTML generator)
//! - Docx (experimental and still very much in-progress; but good enough for "simple" documents without tables, images, etc.)
//!

pub mod htmls;
#[cfg(feature = "docx")]
pub mod docx;
