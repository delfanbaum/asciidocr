use crate::errors::BlockError;
use crate::graph::{
    blocks::{Block, LeafBlock},
    inlines::Inline,
    nodes::{Location, NodeTypes},
};
use crate::scanner::tokens::Token;
use serde::Serialize;

use super::metadata::ElementMetadata;

#[derive(Serialize, Clone, Debug)]
pub struct List {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String,
    pub variant: ListVariant,
    pub items: Vec<Block>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub title: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    pub location: Vec<Location>,
}

impl PartialEq for List {
    fn eq(&self, other: &Self) -> bool {
        self.variant == other.variant
    }
}

impl List {
    /// Creates an empty list; the marker is the first item's marker as written (`*`, `**`, `-`,
    /// `.`, `1.`, ...), without whitespace
    pub fn new(variant: ListVariant, marker: String, location: Vec<Location>) -> Self {
        List {
            name: "list".to_string(),
            node_type: NodeTypes::Block,
            marker: marker.trim().to_string(),
            variant,
            items: vec![],
            title: vec![],
            metadata: None,
            location,
        }
    }

    pub fn add_item(&mut self, item: Block) {
        self.items.push(item)
    }

    /// The marker of the list's first item, as written (`*`, `..`, `1.`, ...)
    pub fn marker(&self) -> &str {
        &self.marker
    }

    /// How the list is numbered, if it is an ordered list
    pub fn numeral_style(&self) -> Option<NumeralStyle> {
        (self.variant == ListVariant::Ordered).then(|| NumeralStyle::from_marker(&self.marker))
    }
}

/// How an ordered list is numbered. Like Asciidoctor, the style follows the marker, not the
/// list's depth: `.` and `1.` are arabic, `..` and `a.` lower alpha, `...` and `i)` lower roman,
/// `....` and `A.` upper alpha, `.....` and `I)` upper roman.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumeralStyle {
    Arabic,
    LowerAlpha,
    LowerRoman,
    UpperAlpha,
    UpperRoman,
}

impl NumeralStyle {
    pub fn from_marker(marker: &str) -> Self {
        match marker {
            ".." => Self::LowerAlpha,
            "..." => Self::LowerRoman,
            "...." => Self::UpperAlpha,
            "....." => Self::UpperRoman,
            _ if marker.len() == 2
                && marker.ends_with('.')
                && marker.starts_with(|c: char| c.is_ascii_alphabetic()) =>
            {
                if marker.starts_with(|c: char| c.is_ascii_lowercase()) {
                    Self::LowerAlpha
                } else {
                    Self::UpperAlpha
                }
            }
            _ if marker.ends_with(')') => {
                if marker.starts_with(['i', 'v', 'x']) {
                    Self::LowerRoman
                } else {
                    Self::UpperRoman
                }
            }
            _ => Self::Arabic,
        }
    }

    /// The name Asciidoctor (and its stylesheet) uses: `arabic`, `loweralpha`, ...
    pub fn name(&self) -> &'static str {
        match self {
            Self::Arabic => "arabic",
            Self::LowerAlpha => "loweralpha",
            Self::LowerRoman => "lowerroman",
            Self::UpperAlpha => "upperalpha",
            Self::UpperRoman => "upperroman",
        }
    }
}

#[derive(Serialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ListVariant {
    Callout,
    Ordered,
    Unordered,
}

#[derive(Serialize, Clone, Debug)]
pub struct ListItem {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    pub marker: String,         // the lexeme with no space
    pub principal: Vec<Inline>, // apparently this can also be optional!
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>, // a LI can have subsequent blocks, too
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    pub location: Vec<Location>,
}

impl PartialEq for ListItem {
    fn eq(&self, other: &Self) -> bool {
        self.marker == other.marker
    }
}

impl ListItem {
    pub fn new(marker: String, location: Vec<Location>) -> Self {
        let trimmed_mark = marker.trim().to_string();
        ListItem {
            name: "listItem".to_string(),
            node_type: NodeTypes::Block,
            marker: trimmed_mark,
            principal: vec![],
            blocks: vec![],
            metadata: None,
            location,
        }
    }

    pub fn add_inline(&mut self, inline: Inline) {
        self.principal.push(inline)
    }

    pub fn principal(&self) -> Vec<Inline> {
        self.principal.clone()
    }

    pub fn blocks(&self) -> Vec<Block> {
        self.blocks.clone()
    }

    pub fn extract_footnotes(
        &mut self,
        count: usize,
        document_id: &str,
    ) -> Result<Vec<Block>, BlockError> {
        let mut local_count = count;
        let mut extracted: Vec<Block> = Vec::new();
        for idx in 0..self.principal.len() {
            if self.principal[idx].is_footnote() {
                local_count += 1;
                let inline_span = self.principal.remove(idx);
                let Inline::InlineSpan(mut footnote) = inline_span else {
                    return Err(BlockError::Footnote("Bad is_footnote match".to_string()));
                };
                // deconstruct it
                let (definition_id, replacement_span, footnote_contents) =
                    footnote.deconstruct_footnote(local_count, document_id)?;
                // add the relevant stuff to the return
                extracted.push(Block::LeafBlock(
                    LeafBlock::new_footnote_def_from_id_and_inlines(
                        definition_id,
                        footnote_contents,
                    )?,
                ));
                // put the reference back where the span was
                self.principal.insert(idx, replacement_span);
            }
        }
        for child in self.blocks.iter_mut() {
            let child_footnoes = child.extract_footnote_definitions(local_count, document_id)?;
            local_count += child_footnoes.len();
            extracted.extend(child_footnoes);
        }
        Ok(extracted)
    }
}

/// Functions essentially the same as a list
#[derive(Serialize, Clone, Debug)]
pub struct DList {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String,
    pub items: Vec<Block>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub title: Vec<Inline>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    pub location: Vec<Location>,
}

impl PartialEq for DList {
    // all dlists are dlists
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl DList {
    pub fn new(location: Vec<Location>) -> Self {
        DList {
            name: "dlist".to_string(),
            node_type: NodeTypes::Block,
            marker: "::".to_string(),
            items: vec![],
            title: vec![],
            metadata: None,
            location,
        }
    }
    pub fn add_item(&mut self, item: Block) {
        self.items.push(item)
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct DListItem {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String, // the lexeme with no space
    /// each term is its own list of inlines, as in the ASG schema: a description can have
    /// several terms (`A::` directly followed by `B:: description`)
    pub terms: Vec<Vec<Inline>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub principal: Vec<Inline>, // apparently this can also be optional!
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>, // a LI can have subsequent blocks, too
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    pub location: Vec<Location>,
}

impl PartialEq for DListItem {
    // all dlistitems are dlist items
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl DListItem {
    pub fn new_from_token(token: Token) -> Self {
        DListItem {
            name: "dlistItem".to_string(),
            node_type: NodeTypes::Block,
            marker: "::".to_string(),
            terms: vec![],
            principal: vec![],
            blocks: vec![],
            metadata: None,
            location: token.locations(),
        }
    }

    /// Adds a term, and reconciles the item's location with it
    pub fn add_term(&mut self, term: Vec<Inline>) {
        for inline in &term {
            self.location = Location::reconcile(self.location.clone(), inline.locations().clone());
        }
        self.terms.push(term)
    }

    pub fn terms(&self) -> Vec<Vec<Inline>> {
        self.terms.clone()
    }

    pub fn principal(&self) -> Vec<Inline> {
        self.principal.clone()
    }

    pub fn add_inline(&mut self, inline: Inline) {
        self.principal.push(inline)
    }

    pub fn locations(&self) -> Vec<Location> {
        self.location.clone()
    }

    pub fn extract_footnotes(
        &mut self,
        count: usize,
        document_id: &str,
    ) -> Result<Vec<Block>, BlockError> {
        let mut local_count = count;
        let mut extracted: Vec<Block> = Vec::new();
        for idx in 0..self.principal.len() {
            if self.principal[idx].is_footnote() {
                local_count += 1;
                let inline_span = self.principal.remove(idx);
                let Inline::InlineSpan(mut footnote) = inline_span else {
                    return Err(BlockError::Footnote("Bad is_footnote match".to_string()));
                };
                // deconstruct it
                let (definition_id, replacement_span, footnote_contents) =
                    footnote.deconstruct_footnote(local_count, document_id)?;
                // add the relevant stuff to the return
                extracted.push(Block::LeafBlock(
                    LeafBlock::new_footnote_def_from_id_and_inlines(
                        definition_id,
                        footnote_contents,
                    )?,
                ));
                // put the reference back where the span was
                self.principal.insert(idx, replacement_span);
            }
        }
        for child in self.blocks.iter_mut() {
            let child_footnoes = child.extract_footnote_definitions(local_count, document_id)?;
            local_count += child_footnoes.len();
            extracted.extend(child_footnoes);
        }
        Ok(extracted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callout_lists_can_be_created() {
        let list = List::new(ListVariant::Callout, "<1>".to_string(), vec![]);
        assert_eq!(list.marker, "<1>");
    }

    #[test]
    fn list_markers_are_kept_as_written() {
        let list = List::new(ListVariant::Ordered, "  .. ".to_string(), vec![]);
        assert_eq!(list.marker, "..");
    }
}
