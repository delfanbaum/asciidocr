use core::panic;
use std::{
    collections::{HashMap, VecDeque},
    fmt::Display,
    iter,
};

use serde::Serialize;

use crate::{
    metadata::ElementMetadata,
    nodes::{Location, NodeTypes},
    tokens::{Token, TokenType},
};

/// Inlines enum containing literals, spans, and references (the latter not implemented)
#[derive(Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum Inline {
    InlineSpan(InlineSpan),
    InlineRef(InlineRef),
    InlineLiteral(InlineLiteral),
    InlineBreak(LineBreak),
}

impl Display for Inline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Inline::InlineSpan(_) => write!(f, "InlineSpan"),
            Inline::InlineRef(_) => write!(f, "InlineRef"),
            Inline::InlineLiteral(_) => write!(f, "InlineLiteral"),
            Inline::InlineBreak(_) => write!(f, "InlineBreak"),
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
            Inline::InlineBreak(_) => matches!(other, Inline::InlineBreak(_)),
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
            Inline::InlineBreak(linebreak) => linebreak.location.clone(),
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
            Inline::InlineBreak(inline) => {
                inline.location = Location::reconcile(inline.location.clone(), other_locs)
            }
        }
    }

    pub fn is_literal(&self) -> bool {
        matches!(self, Inline::InlineLiteral(_))
    }

    pub fn is_macro(&self) -> bool {
        match self {
            Inline::InlineRef(iref) => iref.variant == InlineRefVariant::Link,
            Inline::InlineSpan(span) => span.variant == InlineSpanVariant::Footnote,
            _ => false,
        }
    }

    pub fn is_passthrough(&self) -> bool {
        match self {
            Inline::InlineRef(iref) => iref.variant == InlineRefVariant::Link,
            _ => false,
        }
    }

    pub fn is_open(&self) -> bool {
        match self {
            Inline::InlineSpan(span) => span.open,
            _ => false,
        }
    }

    /// Used for checking if a given inline is just a literal "\n"
    pub fn is_newline(&self) -> bool {
        match self {
            Inline::InlineLiteral(lit) => lit.value == "\n".to_string(),
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
            Inline::InlineBreak(_) => todo!(),
        }
    }

    pub fn extract_literal(&mut self) -> InlineLiteral {
        match &self {
            Inline::InlineLiteral(literal) => literal.clone(),
            _ => panic!("Tried to extract an inline literal from the wrong Inline"),
        }
    }

    pub fn extract_child_inlines(&mut self) -> VecDeque<Inline> {
        match &self {
            Inline::InlineSpan(span) => span.inlines.clone().into(),
            _ => todo!(),
        }
    }

    pub fn produce_literal_from_self(&mut self) -> String {
        match &self {
            Inline::InlineSpan(span) => {
                let mut literal = match span.variant {
                    InlineSpanVariant::Strong => "*".to_string(),
                    InlineSpanVariant::Emphasis => "_".to_string(),
                    InlineSpanVariant::Mark => "#".to_string(),
                    InlineSpanVariant::Code => "`".to_string(),
                    InlineSpanVariant::Superscript => "^".to_string(),
                    InlineSpanVariant::Subscript => "~".to_string(),
                    InlineSpanVariant::Footnote => todo!(), // not applicable
                };
                if span.node_form == InlineSpanForm::Unconstrained {
                    literal = literal
                        .chars()
                        .flat_map(|c| iter::repeat(c).take(2))
                        .collect::<String>();
                }
                literal
            }
            _ => todo!(),
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

    pub fn is_empty(&self) -> bool {
        match self {
            Inline::InlineRef(inline) => inline.inlines.is_empty(),
            Inline::InlineSpan(inline) => inline.inlines.is_empty(),
            Inline::InlineLiteral(inline) => inline.value.is_empty(),
            Inline::InlineBreak(_) => todo!(), // shouldn't ever be called
        }
    }

    pub fn consolidate_locations_from_token(&mut self, token: Token) {
        match self {
            Inline::InlineLiteral(_) => todo!(),
            Inline::InlineSpan(inline) => {
                inline.location = Location::reconcile(inline.location.clone(), token.locations())
            }
            Inline::InlineBreak(_) => todo!(),
            Inline::InlineRef(inline) => {
                inline.location = Location::reconcile(inline.location.clone(), token.locations())
            }
        }
    }

    pub fn add_metadata(&mut self, metadata: ElementMetadata) {
        match self {
            Inline::InlineSpan(span) => span.metadata = Some(metadata),
            _ => panic!("Invalid action: this inline does not take metadata"),
        }
    }

    pub fn close(&mut self) {
        match self {
            Inline::InlineSpan(span) => span.open = false,
            _ => panic!("Invalid action: this inline does not take metadata"),
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
    pub inlines: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    location: Vec<Location>,
    #[serde(skip)]
    pub open: bool,
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
            metadata: None,
            location,
            open: true,
        }
    }

    pub fn inline_span_from_token(token: Token) -> Self {
        match token.token_type() {
            TokenType::Strong => Self::new(
                InlineSpanVariant::Strong,
                InlineSpanForm::Constrained,
                token.locations(),
            ),
            TokenType::Emphasis => Self::new(
                InlineSpanVariant::Emphasis,
                InlineSpanForm::Constrained,
                token.locations(),
            ),
            TokenType::Monospace => Self::new(
                InlineSpanVariant::Code,
                InlineSpanForm::Constrained,
                token.locations(),
            ),
            TokenType::Mark => Self::new(
                InlineSpanVariant::Mark,
                InlineSpanForm::Constrained,
                token.locations(),
            ),
            TokenType::Superscript => Self::new(
                InlineSpanVariant::Superscript,
                InlineSpanForm::Constrained,
                token.locations(),
            ),
            TokenType::Subscript => Self::new(
                InlineSpanVariant::Subscript,
                InlineSpanForm::Constrained,
                token.locations(),
            ),
            // Unconstrained variants
            TokenType::UnconstrainedStrong => Self::new(
                InlineSpanVariant::Strong,
                InlineSpanForm::Unconstrained,
                token.locations(),
            ),
            TokenType::UnconstrainedEmphasis => Self::new(
                InlineSpanVariant::Emphasis,
                InlineSpanForm::Unconstrained,
                token.locations(),
            ),
            TokenType::UnconstrainedMonospace => Self::new(
                InlineSpanVariant::Code,
                InlineSpanForm::Unconstrained,
                token.locations(),
            ),
            TokenType::UnconstrainedMark => Self::new(
                InlineSpanVariant::Mark,
                InlineSpanForm::Unconstrained,
                token.locations(),
            ),
            TokenType::FootnoteMacro => Self::new(
                InlineSpanVariant::Footnote,
                InlineSpanForm::Constrained,
                token.locations(),
            ),
            _ => {
                panic!("Invalid action: tried to create an inline span from an invalid token type")
            }
        }
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
    Superscript,
    Subscript,
    Footnote,
}

#[derive(Serialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InlineSpanForm {
    Constrained,
    Unconstrained,
}

#[derive(Serialize, Clone, Debug)]
pub struct InlineRef {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    variant: InlineRefVariant,
    target: String,
    pub inlines: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<ElementMetadata>,
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
            metadata: None,
            location,
        }
    }

    pub fn new_link_from_token(token: Token) -> Self {
        let mut target = token.text();
        target.pop(); // remove trailing '['
        InlineRef::new(InlineRefVariant::Link, target, token.locations())
    }

    pub fn new_inline_image_from_token(token: Token) -> Self {
        let target_and_attrs = token.text()[6..].to_string(); // after image:
        println!("{:?}", token.text());
        let target: String = target_and_attrs.chars().take_while(|c| c != &'[').collect();
        // get rid of the "[]" chars
        let attributes = target_and_attrs[target.len() + 1..target_and_attrs.len() - 1].to_string();
        if !attributes.is_empty() {
            let mut image_meta = ElementMetadata {
                attributes: HashMap::new(),
                options: vec![],
                roles: vec![],
                inline_metadata: true,
                declared_type: None,
                location: vec![],
            };
            image_meta.process_attributes(attributes.split(',').collect());
            InlineRef {
                name: "ref".to_string(),
                node_type: NodeTypes::Inline,
                variant: InlineRefVariant::Image,
                target,
                inlines: vec![],
                metadata: Some(image_meta),
                location: token.locations(),
            }
        } else {
            InlineRef::new(InlineRefVariant::Image, target, token.locations())
        }
    }

    pub fn is_link(&self) -> bool {
        self.variant == InlineRefVariant::Link
    }

    pub fn add_text_from_token(&mut self, token: Token) {
        let inline_literal = Inline::InlineLiteral(InlineLiteral::new_text_from_token(&token));
        if let Some(last_inline) = self.inlines.last_mut() {
            match last_inline {
                Inline::InlineSpan(span) => span.add_inline(inline_literal),
                Inline::InlineLiteral(prior_literal) => prior_literal.add_text_from_token(&token),
                _ => panic!("Can't add text to last token in this context"),
            }
        } else {
            self.inlines.push(inline_literal)
        }
    }
}

#[derive(Serialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InlineRefVariant {
    Link,
    Xref,
    Image,
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

    pub fn prepend_to_value(&mut self, value: String, value_locations: Vec<Location>) {
        // add the value
        self.value.insert_str(0, &value);
        // update the locations
        self.location = Location::reconcile(self.location.clone(), value_locations);
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum InlineLiteralName {
    Text,
    Charref,
    Raw,
}

#[derive(Serialize, Clone, Debug)]
pub struct LineBreak {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes, // always "string"
    location: Vec<Location>,
}

impl LineBreak {
    pub fn new_from_token(token: Token) -> Self {
        LineBreak {
            name: "linebreak".to_string(),
            node_type: NodeTypes::Inline,
            location: token.locations(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tokens::Token;

    use super::InlineRef;

    #[test]
    fn image_from_token() {
        let token = Token::new(
            crate::tokens::TokenType::InlineImageMacro,
            "image:path/to/img.png[]".to_string(),
            Some("image:path/to/img.png[]".to_string()),
            1,
            1,
            23,
        );
        let img_ref = InlineRef::new_inline_image_from_token(token);
        assert_eq!(img_ref.target, "path/to/img.png".to_string())
    }
    #[test]
    fn image_from_token_title() {
        let token = Token::new(
            crate::tokens::TokenType::InlineImageMacro,
            "image:path/to/img.png[title=Pause]".to_string(),
            Some("image:path/to/img.png[title=Pause]".to_string()),
            1,
            1,
            23,
        );
        let img_ref = InlineRef::new_inline_image_from_token(token);
        assert_eq!(img_ref.target, "path/to/img.png".to_string());
        assert!(img_ref.metadata.is_some());
        if let Some(metadata) = img_ref.metadata {
            assert!(metadata.inline_metadata);
            assert_eq!(
                metadata.attributes.get("title").unwrap(),
                &"Pause".to_string()
            )
        }
    }
}
