use core::panic;
use std::{
    collections::{HashMap, VecDeque},
    fmt::Display,
};

use log::warn;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

use super::{
    metadata::ElementMetadata,
    nodes::{Location, NodeTypes},
    substitutions::CHARREF_MAP,
};
use crate::scanner::tokens::{Token, TokenType};

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

    /// Whether this is a macro still waiting for its closing `]`: a footnote, or a link macro
    /// (but not a bare URL, which is also a link)
    pub fn is_open_macro(&self) -> bool {
        match self {
            Inline::InlineRef(iref) => iref.open,
            _ => self.is_macro(),
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
            Inline::InlineLiteral(lit) => lit.value == *"\n",
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
            Inline::InlineRef(iref) => {
                let mut values = String::new();
                for inline in &iref.inlines {
                    values.push_str(&inline.extract_values_to_string())
                }
                values
            }
            Inline::InlineBreak(_) => String::new(), // has no value
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
            Inline::InlineSpan(span) => span.extract_span_inlines(),
            Inline::InlineRef(ref_) => ref_.inlines.clone().into(),
            Inline::InlineBreak(_) | Inline::InlineLiteral(_) => {
                // This really shouldn't ever be called
                let mut v = VecDeque::new();
                v.push_front(self.clone());
                v
            }
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
                    InlineSpanVariant::Footnote => "".to_string(),
                };
                if span.node_form == InlineSpanForm::Unconstrained {
                    literal = literal
                        .chars()
                        .flat_map(|c| std::iter::repeat_n(c, 2))
                        .collect::<String>();
                }
                literal
            }
            _ => unreachable!(),
        }
    }

    /// The children of a span that was never closed, with the span's markup characters put back in
    /// front of them as text: without a closing mark, the opening one was never markup at all.
    pub fn children_with_markup_as_text(&mut self) -> VecDeque<Inline> {
        let markup = self.produce_literal_from_self();
        let locations = self.locations();
        let mut children = self.extract_child_inlines();
        match children.front_mut() {
            Some(Inline::InlineLiteral(literal))
                if matches!(literal.name, InlineLiteralName::Text) =>
            {
                literal.prepend_to_value(markup, locations);
            }
            _ => {
                // the markup is as wide as it is long, and starts where the span starts
                let mut location = locations;
                location.truncate(1);
                if let Some(start) = location.first().cloned() {
                    let mut end = start.clone();
                    end.col += markup.chars().count().saturating_sub(1);
                    location = vec![start, end];
                }
                children.push_front(Inline::InlineLiteral(InlineLiteral::new(
                    InlineLiteralName::Text,
                    markup,
                    location,
                )));
            }
        }
        children
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
            Inline::InlineBreak(_) => unreachable!(),
        }
    }

    pub fn consolidate_locations_from_token(&mut self, token: Token) {
        match self {
            Inline::InlineLiteral(_) => unreachable!(),
            Inline::InlineSpan(inline) => {
                inline.location = Location::reconcile(inline.location.clone(), token.locations())
            }
            Inline::InlineBreak(_) => unreachable!(),
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

    pub fn attempt_xref_standardization(&mut self, id_hash: &HashMap<String, Vec<Inline>>) {
        if let Inline::InlineRef(iref) = self
            && matches!(iref.variant, InlineRefVariant::Xref)
        {
            if let Some(ref_text) = id_hash.get(&iref.target) {
                iref.inlines = ref_text.clone()
            } else {
                warn!("Unable to find xref: {}", iref.target)
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
            TokenType::Literal => Self::new(
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
            TokenType::UnconstrainedLiteral => Self::new(
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
            TokenType::CodeCallout => {
                let metadata = ElementMetadata::new_with_role("conum".into(), token.line);
                let mut code_span = Self::new(
                    InlineSpanVariant::Strong,
                    InlineSpanForm::Constrained,
                    token.locations(),
                );
                code_span.metadata = Some(metadata);
                code_span
                    .inlines
                    .push(Inline::InlineLiteral(InlineLiteral::new(
                        InlineLiteralName::Text,
                        format!("({})", &token.text()[1..token.text().len() - 1]),
                        token.locations(),
                    )));
                code_span
            }
            _ => {
                panic!("Invalid action: tried to create an inline span from an invalid token type")
            }
        }
    }

    pub fn add_inline(&mut self, inline: Inline) {
        // update the locations
        self.location = Location::reconcile(self.location.clone(), inline.locations());
        if let Some(Inline::InlineSpan(last_span)) = self.inlines.last_mut() {
            if last_span.open {
                last_span.add_inline(inline);
            } else {
                self.inlines.push(inline);
            }
        } else {
            // combine literals if necessary
            if let Inline::InlineLiteral(new_literal) = &inline
                && let Some(Inline::InlineLiteral(prior_literal)) = self.inlines.last_mut()
                && prior_literal.can_merge_with(new_literal)
            {
                prior_literal.add_text_from_inline_literal(inline);
                return;
            }
            self.inlines.push(inline);
        }
    }

    // extracts the inlines inside the span, closing any open (dangling) spans that may be
    // inside of it
    fn extract_span_inlines(&self) -> VecDeque<Inline> {
        let mut children = VecDeque::new();
        for inline in self.inlines.iter() {
            // handle any open spans
            if inline.is_open() {
                let mut working_inline = inline.clone();
                let inline_children = working_inline.children_with_markup_as_text();
                children.extend(inline_children);
            } else {
                children.push_back(inline.clone());
            }
        }
        children
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
    pub variant: InlineRefVariant,
    pub target: String,
    pub inlines: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    location: Vec<Location>,
    /// Whether the ref still takes the text that follows it: only a link macro does, until its
    /// closing `]`. A bare URL, an xref or an image is complete as soon as it is scanned.
    #[serde(skip)]
    pub open: bool,
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
            open: false,
        }
    }

    pub fn new_xref_from_token(token: Token) -> Self {
        let target = token.text()[2..token.text().len() - 2].to_string();
        InlineRef::new(InlineRefVariant::Xref, target, token.locations())
    }

    pub fn new_link_from_macro_token(token: Token) -> Self {
        let mut target = token.text();
        target.pop(); // remove trailing '['
        InlineRef {
            open: true,
            ..InlineRef::new(InlineRefVariant::Link, target, token.locations())
        }
    }

    pub fn new_link_from_token(token: Token) -> Self {
        let target = token.text();
        InlineRef::new(InlineRefVariant::Link, target, token.locations())
    }

    pub fn new_inline_image(
        target: String,
        metadata: Option<ElementMetadata>,
        location: Vec<Location>,
    ) -> Self {
        if metadata.is_some() {
            InlineRef {
                name: "ref".to_string(),
                node_type: NodeTypes::Inline,
                variant: InlineRefVariant::Image,
                target,
                inlines: vec![],
                metadata,
                location,
                open: false,
            }
        } else {
            InlineRef::new(InlineRefVariant::Image, target, location)
        }
    }

    pub fn is_link(&self) -> bool {
        self.variant == InlineRefVariant::Link
    }

    pub fn add_text_from_token(&mut self, token: Token) {
        self.text_from_token(token, false);
    }

    pub fn pass_text_from_token(&mut self, token: Token) {
        self.text_from_token(token, true);
    }

    fn text_from_token(&mut self, token: Token, pass: bool) {
        let inline_literal = Inline::InlineLiteral(InlineLiteral::new_text_from_token_pass(&token));
        if let Some(last_inline) = self.inlines.last_mut() {
            match last_inline {
                Inline::InlineSpan(span) => span.add_inline(inline_literal),
                Inline::InlineLiteral(prior_literal) => match pass {
                    true => prior_literal.pass_text_from_token(&token),
                    false => prior_literal.add_text_from_token(&token),
                },
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
    pub name: InlineLiteralName,
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

    pub fn new_charref_from_token(token: &Token) -> Self {
        InlineLiteral::new(InlineLiteralName::Charref, token.text(), token.locations())
    }

    /// Passes the lexeme through, e.g., when we're in a source or literal block
    pub fn new_text_from_token_pass(token: &Token) -> Self {
        InlineLiteral::new(
            InlineLiteralName::Text,
            token.lexeme.clone(),
            token.locations(),
        )
    }

    /// Text that was passed through an inline passthrough macro (e.g., `pass:[<b>text</b>]`), which
    /// backends output as is.
    pub fn new_raw_from_token(token: &Token) -> Self {
        InlineLiteral::new(
            InlineLiteralName::Raw,
            token.lexeme.clone(),
            token.locations(),
        )
    }

    /// Add text and reconcile location information from a given (text) token
    pub fn add_text_from_token(&mut self, token: &Token) {
        self.text_from_token(token, false);
    }

    /// Add text and reconcile location information from a given (text) token
    pub fn pass_text_from_token(&mut self, token: &Token) {
        self.text_from_token(token, true);
    }

    /// Lazy DRY function for the above
    fn text_from_token(&mut self, token: &Token, pass_through: bool) {
        match pass_through {
            true => self.value.push_str(&token.lexeme.clone()),
            false => self.value.push_str(&token.text()),
        }
        self.location = Location::reconcile(self.location.clone(), token.locations());
    }

    /// Whether the other literal's text can simply be appended to this one. Passthrough text only
    /// merges with other passthrough text, because backends output it as is while escaping
    /// everything else.
    pub fn can_merge_with(&self, other: &InlineLiteral) -> bool {
        matches!(self.name, InlineLiteralName::Raw) == matches!(other.name, InlineLiteralName::Raw)
    }

    /// Add test from inline literals; should only really be used in reconciling multi-line spans
    pub fn add_text_from_inline_literal(&mut self, inline: Inline) {
        match inline {
            Inline::InlineLiteral(ref literal) => self.value.push_str(&literal.value),
            _ => panic!("Can't add test from this kind of inline: {:?}", inline),
        }
        self.location = Location::reconcile(self.location.clone(), inline.locations().clone());
    }

    /// Splits off the text after the last newline, i.e., the text on this literal's last line
    /// (the term of a description list item, say, whose line the `::` ends). The newline is
    /// dropped. Returns `None` if the text has no newline.
    pub fn split_off_last_line(&mut self) -> Option<InlineLiteral> {
        let newline_idx = self.value.rfind('\n')?;
        let (Some(start), Some(end)) = (self.location.first(), self.location.last()) else {
            return None;
        };
        let (start, end) = (start.clone(), end.clone());
        let last_line = self.value.split_off(newline_idx + 1);
        self.value.truncate(newline_idx);
        // the text before the newline now ends a line earlier, at its last character
        let end_col = match self.value.rfind('\n') {
            Some(idx) => self.value[idx + 1..].chars().count(),
            None => (start.col + self.value.chars().count()).saturating_sub(1),
        };
        self.location = vec![
            start,
            Location::new(end.line.saturating_sub(1), end_col.max(1), end.file.clone()),
        ];
        // and the last line starts at its first column
        let last_line_start = Location::new(end.line, 1, end.file.clone());
        Some(InlineLiteral::new(
            self.name.clone(),
            last_line,
            vec![last_line_start, end],
        ))
    }

    /// Drops the whitespace the text starts with (the indentation of its line), and moves its
    /// start location past it
    pub fn trim_start(&mut self) {
        let indent = self.value.len() - self.value.trim_start().len();
        if indent > 0 {
            self.value.drain(..indent);
            if let Some(start) = self.location.first_mut() {
                start.col += indent;
            }
        }
    }

    pub fn combine_literals(&mut self, inline: InlineLiteral) {
        self.value.push_str(&inline.value());
        self.location = Location::reconcile(self.location.clone(), inline.location.clone());
    }

    /// Drops the comment markup (`//`, `#`, `--` or `;;`) that ends this text, which is how code
    /// callouts are usually introduced (e.g., `int x; // <1>`)
    pub fn strip_trailing_comment_markup(&mut self) {
        static TRAILING_COMMENT: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(?://|#|--|;;)[ \t]*$").unwrap());
        self.value = TRAILING_COMMENT.replace(&self.value, "").to_string();
    }

    pub fn prepend_to_value(&mut self, value: String, value_locations: Vec<Location>) {
        // add the value
        self.value.insert_str(0, &value);
        // update the locations
        self.location = Location::reconcile(self.location.clone(), value_locations);
    }

    pub fn value(&self) -> String {
        self.value.clone()
    }

    pub fn value_or_refd_char(&self) -> String {
        // In HTML these just get passed through, but for certain outputs we want to do replacements. This
        // is obviously an incomplete table.
        if matches!(self.name, InlineLiteralName::Charref) {
            let charrefs = CHARREF_MAP.lock().unwrap();
            match charrefs.get(&self.value) {
                Some(entity) => entity.to_string(),
                None => self.value.clone(),
            }
        } else {
            self.value.clone()
        }
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

    use crate::scanner::tokens::{Token, TokenType};
    use crate::utils::target_and_attrs_from_token;

    use super::*;
    use rstest::rstest;

    #[test]
    fn xref_from_token() {
        let reference = "<<foo>>".to_string();
        let token = Token {
            token_type: TokenType::CrossReference,
            lexeme: reference.clone(),
            literal: Some(reference),
            line: 1,
            startcol: 1,
            endcol: 1,
            file_stack: vec![],
        };
        let inline = InlineRef::new_xref_from_token(token);
        assert_eq!(inline.variant, InlineRefVariant::Xref);
        assert_eq!(inline.target, "foo".to_string());
    }

    #[test]
    fn image_from_token() {
        let token = Token::new_default(
            TokenType::InlineImageMacro,
            "image:path/to/img.png[]".to_string(),
            Some("image:path/to/img.png[]".to_string()),
            1,
            1,
            23,
        );
        let (target, metadata) = target_and_attrs_from_token(&token);
        let img_ref = InlineRef::new_inline_image(target, metadata, token.locations());
        assert_eq!(img_ref.target, "path/to/img.png".to_string())
    }

    #[test]
    fn image_from_token_title() {
        let token = Token::new_default(
            TokenType::InlineImageMacro,
            "image:path/to/img.png[title=Pause]".to_string(),
            Some("image:path/to/img.png[title=Pause]".to_string()),
            1,
            1,
            23,
        );
        let (target, metadata) = target_and_attrs_from_token(&token);
        let img_ref = InlineRef::new_inline_image(target, metadata, token.locations());
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

    #[rstest]
    #[case::nbsp("&nbsp;", " ")]
    #[case::mdash("&mdash;", "—")]
    #[case::ndash("&ndash;", "—")]
    #[case::dollar("&dollar;", "$")]
    #[case::amp("&amp;", "&")]
    #[case::gt("&gt;", ">")]
    #[case::lt("&lt;", "<")]
    #[case::equals("&equals;", "=")]
    #[case::plus("&plus;", "+")]
    fn test_value_or_refd_char(#[case] entity: &str, #[case] replacement: &str) {
        let inline = InlineLiteral::new(
            InlineLiteralName::Charref,
            entity.into(),
            vec![Location::default(), Location::default()],
        );
        assert_eq!(inline.value_or_refd_char(), replacement)
    }

    /// "def\nmore\nTerm", from (1, 5) to (3, 4): "Term" is on line 3, and the rest ends at the
    /// end of line 2
    #[test]
    fn split_off_last_line() {
        let mut literal = InlineLiteral::new(
            InlineLiteralName::Text,
            "def\nmore\nTerm".to_string(),
            vec![Location::new(1, 5, vec![]), Location::new(3, 4, vec![])],
        );
        let last_line = literal.split_off_last_line().unwrap();
        assert_eq!(last_line.value(), "Term");
        assert_eq!(
            last_line.location,
            vec![Location::new(3, 1, vec![]), Location::new(3, 4, vec![])]
        );
        assert_eq!(literal.value(), "def\nmore");
        assert_eq!(
            literal.location,
            vec![Location::new(1, 5, vec![]), Location::new(2, 4, vec![])]
        );
    }

    #[test]
    fn split_off_last_line_without_a_newline() {
        let mut literal = InlineLiteral::new(
            InlineLiteralName::Text,
            "Term".to_string(),
            vec![Location::new(1, 1, vec![]), Location::new(1, 4, vec![])],
        );
        assert!(literal.split_off_last_line().is_none());
        assert_eq!(literal.value(), "Term");
    }
}
