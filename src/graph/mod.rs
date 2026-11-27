//! This module contains elements of the Abstract Syntax Graph used to represent an Asciidoc
//! document, roughly meaning to follow the "official" schema:
//! <https://gitlab.eclipse.org/eclipse/asciidoc-lang/asciidoc-lang/-/blob/main/asg/schema.json>
//!
//! An ASG is made up of Blocks, which are in turn made up of Inlines. All elements, including the
//! graph itself, are serializeable with `serde`, giving us JSON output (as required by the
//! Technology Compatibility Kit) and making certain templating tasks simpler.

pub mod asg;
pub mod blocks;
pub(crate) mod ids;
pub mod inlines;
pub mod lists;
pub mod metadata;
pub mod nodes;
// only the docx backend (and a test) needs the characters that references stand for
#[cfg_attr(not(feature = "docx"), allow(dead_code))]
pub(crate) mod substitutions;
