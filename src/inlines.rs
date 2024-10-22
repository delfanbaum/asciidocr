use core::panic;
use std::fmt::Display;

use serde::Serialize;

use crate::{
    nodes::{Location, NodeTypes},
    tokens::Token,
};

/// Inlines enum containing literals, spans, and references (the latter not implemented)
#[derive(Serialize, Clone, Debug)]
#[serde(untagged)]
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

    pub fn is_literal(&self) -> bool {
        matches!(self, Inline::InlineLiteral(_))
    }

    pub fn extract_values_to_string(&self) -> String {
        match &self {
            Inline::InlineLiteral(literal) => literal.value.clone(),
            Inline::InlineSpan(span) => {
                let mut values = String::new();
                for inline in &span.inlines {
                    values.push_str(&inline.extract_values_to_string())
                }
                values
            }
            Inline::InlineRef(_) => todo!(),
        }
    }

    pub fn extract_literal(&mut self) -> InlineLiteral {
        match &self {
            Inline::InlineLiteral(literal) => literal.clone(),
            _ => panic!("Tried to extract an inline literal from the wrong Inline"),
        }
    }

    pub fn prepend_literal(&mut self, preceding_literal: InlineLiteral) {
        match self {
            Inline::InlineLiteral(literal) => {
                // combine values
                literal.value.insert_str(0, &preceding_literal.value);
                // combine locations
                if let Some(end_location) = literal.location.pop() {
                    literal.location = vec![preceding_literal.location[0].clone(), end_location]
                } else {
                    literal.location = preceding_literal.location.clone()
                }
            }
            _ => panic!("Tried to prepend an inline literal to the wrong Inline"),
        }
    }

    pub fn trim(&mut self) {
        if let Inline::InlineLiteral(inline) = self {
            inline.value = inline.value.trim().to_string();
            println!("{:?}", inline.value)
        }

    }
}

#[derive(Serialize, Clone, Debug)]
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
    pub fn add_inline(&mut self, inline: Inline) {
        self.location = Location::reconcile(self.location.clone(), inline.locations());
        self.inlines.push(inline);
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InlineSpanVariant {
    Strong,
    Emphasis,
    Code,
    Mark,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InlineSpanForm {
    Constrainted,
    Unconstrainted,
}

// REFS NOT CURRENTLY SUPPORTED, this is just saving future work
#[derive(Serialize, Clone, Debug)]
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

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InlineRefVariant {
    Link,
    Xref,
}

#[derive(Serialize, Clone, Debug)]
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

    pub fn add_text_from_token(&mut self, token: &Token) {
        self.value.push_str(&token.text());
        self.location = Location::reconcile(self.location.clone(), token.locations());
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InlineLiteralName {
    Text,
    Charref,
    Raw,
}
