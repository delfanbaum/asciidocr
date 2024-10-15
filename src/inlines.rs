use core::panic;
use std::fmt::Display;

use serde::Serialize;

use crate::{
    nodes::{Location, NodeTypes},
    tokens::Token,
};

#[derive(Serialize)]
pub enum Inline {
    InlineSpan(InlineSpan),
    InlineRef(InlineRef),
    InlineLiteral(InlineLiteral),
}

impl Display for Inline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Inline::InlineSpan(_) => write!(f, "InlineSpan"),
            Inline::InlineRef(_) => write!(f, "InlineRef"),
            Inline::InlineLiteral(_) => write!(f, "InlineLiteral"),
        }
    }
}

impl Inline {
    pub fn push_inline(&mut self, child: Inline) {
        match self {
            Inline::InlineSpan(span) => span.inlines.push(child),
            _ => panic!("Inlines of type {} do not accept child inlines!", &self),
        }
    }
    pub fn locations(&self) -> Vec<Location> {
        match &self {
            Inline::InlineSpan(span) => span.location.clone(),
            Inline::InlineRef(iref) => iref.location.clone(),
            Inline::InlineLiteral(lit) => lit.location.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct InlineSpan {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    variant: InlineSpanVariant,
    node_form: InlineSpanForm,
    inlines: Vec<Inline>,
    location: Vec<Location>,
}

impl InlineSpan {
    pub fn new(
        variant: InlineSpanVariant,
        node_form: InlineSpanForm,
        location: Vec<Location>,
    ) -> Self {
        InlineSpan {
            name: "span".to_string(),
            node_type: NodeTypes::Inline,
            variant,
            node_form,
            inlines: vec![],
            location,
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
    pub fn new(variant: InlineRefVariant, target: String, location: Vec<Location>) -> Self {
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
    pub fn new(name: InlineLiteralName, value: String, location: Vec<Location>) -> Self {
        InlineLiteral {
            name,
            node_type: NodeTypes::String,
            value,
            location,
        }
    }

    pub fn new_text_from_token(token: &Token) -> Self {
        InlineLiteral::new(InlineLiteralName::Text, token.text(), token.locations())
    }
}

#[derive(Serialize)]
pub enum InlineLiteralName {
    Text,
    Charref,
    Raw,
}
