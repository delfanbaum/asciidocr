//! This module contains elements of the Abstract Syntax Graph used to represent an Asciidoc
//! document, roughly meaning to follow the "official" schema:
//! <https://gitlab.eclipse.org/eclipse/asciidoc-lang/asciidoc-lang/-/blob/main/asg/schema.json>
//!
//! An ASG is made up of Blocks, which are in turn made up of Inlines. All elements, including the
//! graph itself, are serializeable with `serde`, giving us JSON output (as required by the
//! Technology Compatibility Kit) and making certain templating tasks simpler.

use serde::Serialize;

pub mod asg;
pub mod blocks;
pub mod header;
pub(crate) mod ids;
pub mod inlines;
pub mod lists;
pub mod location;
pub mod metadata;
pub(crate) mod substitutions;
mod word_count;

/// The `type` of a node in the ASG
#[derive(Clone, Copy, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum NodeTypes {
    /// A block
    Block,
    /// An inline element
    Inline,
    /// A string of text
    String,
}
