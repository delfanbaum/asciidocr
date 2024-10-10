use serde::Serialize;

use crate::nodes::{Location, NodeTypes};

#[derive(Serialize)]
pub enum Inline {
    InlineSpan(InlineSpan),
    InlineRef(InlineRef),
    InlineLiteral(InlineLiteral),
}

#[derive(Serialize)]
pub struct InlineSpan {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    variant: InlineSpanVariant,
    node_form: InlineSpanForm,
    inlines: Vec<Inline>,
}

impl InlineSpan {
    fn new(variant: InlineSpanVariant, node_form: InlineSpanForm) -> Self {
        InlineSpan {
            name: "span".to_string(),
            node_type: NodeTypes::Inline,
            variant,
            node_form,
            inlines: vec![],
        }
    }
}

#[derive(Serialize)]
pub enum InlineSpanVariant {
    Strong,
    Emphasis,
    Code,
    Mark,
}

#[derive(Serialize)]
pub enum InlineSpanForm {
    Constrainted,
    Unconstrainted,
}

// REFS NOT CURRENTLY SUPPORTED, this is just saving future work
#[derive(Serialize)]
pub struct InlineRef {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    variant: InlineRefVariant,
    target: String,
    inlines: Vec<Inline>,
    location: Vec<Location>,
}

impl InlineRef {
    fn new(variant: InlineRefVariant, target: String, location: Vec<Location>) -> Self {
        InlineRef {
            name: "ref".to_string(),
            node_type: NodeTypes::Inline,
            variant,
            target,
            inlines: vec![],
            location,
        }
    }
}

#[derive(Serialize)]
pub enum InlineRefVariant {
    Link,
    Xref,
}

#[derive(Serialize)]
pub struct InlineLiteral {
    name: InlineLiteralName,
    #[serde(rename = "type")]
    node_type: NodeTypes, // always "string"
    value: String,
    location: Vec<Location>,
}

impl InlineLiteral {
    fn new(name: InlineLiteralName, value: String, location: Vec<Location>) -> Self {
        InlineLiteral {
            name,
            node_type: NodeTypes::String,
            value,
            location,
        }
    }
}

#[derive(Serialize)]
pub enum InlineLiteralName {
    Text,
    Charref,
    Raw,
}
