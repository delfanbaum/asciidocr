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

impl PartialEq for Inline {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Inline::InlineSpan(span) => match other {
                Inline::InlineSpan(other_span) => {
                    span.variant == other_span.variant && span.node_form == other_span.node_form
                }
                _ => false,
            },
            Inline::InlineRef(_) => matches!(other, Inline::InlineRef(_)),
            Inline::InlineLiteral(_) => matches!(other, Inline::InlineLiteral(_)),
        }
    }
}

impl Inline {
    pub fn push_inline(&mut self, child: Inline) {
        match self {
            Inline::InlineSpan(span) => span.inlines.push(child),
            Inline::InlineRef(iref) => iref.inlines.push(child),
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

    pub fn reconcile_locations(&mut self, other_locs: Vec<Location>) {
        match self {
            Inline::InlineSpan(inline) => {
                inline.location = Location::reconcile(inline.location.clone(), other_locs)
            }
            Inline::InlineRef(inline) => {
                inline.location = Location::reconcile(inline.location.clone(), other_locs)
            }
            Inline::InlineLiteral(inline) => {
                inline.location = Location::reconcile(inline.location.clone(), other_locs)
            }
        }
    }

    pub fn is_literal(&self) -> bool {
        matches!(self, Inline::InlineLiteral(_))
    }

    pub fn is_macro(&self) -> bool {
        match self {
            Inline::InlineRef(iref) => {
                iref.variant == InlineRefVariant::Link
            }
            _ => false,
        }
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
    pub fn consolidate_locations_from_token(&mut self, token: Token) {
        match self {
            Inline::InlineLiteral(_) => todo!(),
            Inline::InlineSpan(_) => todo!(),
            Inline::InlineRef(iref) =>  {
                iref.location = Location::reconcile(iref.location.clone(), token.locations())
            }
        }

    }
}

#[derive(Serialize, Clone, Debug)]
pub struct InlineSpan {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    pub variant: InlineSpanVariant,
    #[serde(rename = "form")]
    pub node_form: InlineSpanForm,
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

    pub fn new_emphasis_span(token: Token) -> Self {
        Self::new(
            InlineSpanVariant::Emphasis,
            InlineSpanForm::Constrained,
            token.locations(),
        )
    }
    pub fn new_strong_span(token: Token) -> Self {
        Self::new(
            InlineSpanVariant::Strong,
            InlineSpanForm::Constrained,
            token.locations(),
        )
    }
    pub fn new_code_span(token: Token) -> Self {
        Self::new(
            InlineSpanVariant::Code,
            InlineSpanForm::Constrained,
            token.locations(),
        )
    }
    pub fn new_mark_span(token: Token) -> Self {
        Self::new(
            InlineSpanVariant::Mark,
            InlineSpanForm::Constrained,
            token.locations(),
        )
    }

    pub fn add_inline(&mut self, inline: Inline) {
        // update the locations
        self.location = Location::reconcile(self.location.clone(), inline.locations());
        // combine literals if necessary
        if matches!(inline, Inline::InlineLiteral(_)) {
            if let Some(Inline::InlineLiteral(prior_literal)) = self.inlines.last_mut() {
                prior_literal.add_text_from_inline_literal(inline);
                return;
            }
        }
        self.inlines.push(inline);
    }
}

#[derive(Serialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InlineSpanVariant {
    Strong,
    Emphasis,
    Code,
    Mark,
}

#[derive(Serialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InlineSpanForm {
    Constrained,
    Unconstrainted,
}

#[derive(Serialize, Clone, Debug)]
pub struct InlineRef {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    variant: InlineRefVariant,
    target: String,
    pub inlines: Vec<Inline>,
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

    pub fn new_link_from_token(token: Token) -> Self {
        let mut target = token.text();
        target.pop(); // remove trailing '['
        InlineRef::new(InlineRefVariant::Link, target, token.locations())
    }

    pub fn is_link(&self) -> bool {
        self.variant == InlineRefVariant::Link
    }
}

#[derive(Serialize, PartialEq, Clone, Debug)]
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

    /// Add text and reconcile location information from a given (text) token
    pub fn add_text_from_token(&mut self, token: &Token) {
        self.value.push_str(&token.text());
        self.location = Location::reconcile(self.location.clone(), token.locations());
    }

    /// Add test from inline literals; should only really be used in reconciling multi-line spans
    pub fn add_text_from_inline_literal(&mut self, inline: Inline) {
        match inline {
            Inline::InlineLiteral(ref literal) => self.value.push_str(&literal.value),
            _ => panic!("Can't add test from this kind of inline: {:?}", inline),
        }
        self.location = Location::reconcile(self.location.clone(), inline.locations().clone());
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InlineLiteralName {
    Text,
    Charref,
    Raw,
}
