//! Lists and description lists, and their items

use crate::graph::{
    blocks::Block,
    inlines::Inline,
    nodes::{Location, NodeTypes},
};
use crate::scanner::Token;
use serde::Serialize;

use super::metadata::ElementMetadata;

/// An ordered or unordered list
#[derive(Serialize, Clone, Debug)]
pub struct List {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String,
    /// Whether the items are numbered
    pub variant: ListVariant,
    /// The items ([`Block::ListItem`]s)
    pub items: Vec<Block>,
    /// The block title above the list, if any
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub title: Vec<Inline>,
    /// The list's attributes, roles and options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    /// Where the list starts and ends in the source
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
    pub(crate) fn new(variant: ListVariant, marker: String, location: Vec<Location>) -> Self {
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

    pub(crate) fn add_item(&mut self, item: Block) {
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
#[non_exhaustive]
pub enum NumeralStyle {
    /// 1, 2, 3
    Arabic,
    /// a, b, c
    LowerAlpha,
    /// i, ii, iii
    LowerRoman,
    /// A, B, C
    UpperAlpha,
    /// I, II, III
    UpperRoman,
}

impl NumeralStyle {
    /// The style for a list whose items have this marker
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

/// The kinds of list
#[derive(Serialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ListVariant {
    /// A list of source code callouts (the parser doesn't produce these yet)
    Callout,
    /// Numbered or lettered items
    Ordered,
    /// Bulleted items
    Unordered,
}

/// An item of an ordered or unordered list
#[derive(Serialize, Clone, Debug)]
pub struct ListItem {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    /// The item's marker, as written (`*`, `..`, `1.`)
    pub marker: String, // the lexeme with no space
    /// The item's own text
    pub principal: Vec<Inline>, // apparently this can also be optional!
    /// The blocks attached to the item, nested lists included
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>, // a LI can have subsequent blocks, too
    /// The item's attributes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    /// Where the item starts and ends in the source
    pub location: Vec<Location>,
}

impl PartialEq for ListItem {
    fn eq(&self, other: &Self) -> bool {
        self.marker == other.marker
    }
}

impl ListItem {
    pub(crate) fn new(marker: String, location: Vec<Location>) -> Self {
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

    pub(crate) fn add_inline(&mut self, inline: Inline) {
        self.principal.push(inline)
    }

    /// The item's own text
    pub fn principal(&self) -> Vec<Inline> {
        self.principal.clone()
    }

    /// The blocks attached to the item
    pub fn blocks(&self) -> Vec<Block> {
        self.blocks.clone()
    }
}

/// Functions essentially the same as a list
#[derive(Serialize, Clone, Debug)]
pub struct DList {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String,
    /// The items ([`Block::DListItem`]s)
    pub items: Vec<Block>,
    /// The block title above the list, if any
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub title: Vec<Inline>,
    /// The list's attributes, roles and options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    /// Where the list starts and ends in the source
    pub location: Vec<Location>,
}

impl PartialEq for DList {
    // all dlists are dlists
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl DList {
    /// The marker of the list's items, as written (e.g., `::`)
    pub fn marker(&self) -> &str {
        &self.marker
    }

    /// Creates an empty description list; the marker is its first item's (`::`, `;;`, ...)
    pub(crate) fn new(marker: String, location: Vec<Location>) -> Self {
        DList {
            name: "dlist".to_string(),
            node_type: NodeTypes::Block,
            marker: marker.trim().to_string(),
            items: vec![],
            title: vec![],
            metadata: None,
            location,
        }
    }
    pub(crate) fn add_item(&mut self, item: Block) {
        self.items.push(item)
    }
}

/// An item of a description list: one or more terms and their description
#[derive(Serialize, Clone, Debug)]
pub struct DListItem {
    name: String,
    #[serde(rename = "type")]
    node_type: NodeTypes,
    marker: String, // the lexeme with no space
    /// each term is its own list of inlines, as in the ASG schema: a description can have
    /// several terms (`A::` directly followed by `B:: description`)
    pub terms: Vec<Vec<Inline>>,
    /// The description's text
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub principal: Vec<Inline>, // apparently this can also be optional!
    /// The blocks attached to the item
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<Block>, // a LI can have subsequent blocks, too
    /// The item's attributes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ElementMetadata>,
    /// Where the item starts and ends in the source
    pub location: Vec<Location>,
}

impl PartialEq for DListItem {
    // all dlistitems are dlist items
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl DListItem {
    /// The item's marker, as written (e.g., `::`)
    pub fn marker(&self) -> &str {
        &self.marker
    }

    pub(crate) fn new_from_token(token: Token) -> Self {
        DListItem {
            name: "dlistItem".to_string(),
            node_type: NodeTypes::Block,
            marker: token.lexeme.trim().to_string(),
            terms: vec![],
            principal: vec![],
            blocks: vec![],
            metadata: None,
            location: token.locations(),
        }
    }

    /// Adds a term, and reconciles the item's location with it
    pub(crate) fn add_term(&mut self, term: Vec<Inline>) {
        for inline in &term {
            self.location = Location::reconcile(self.location.clone(), inline.locations().clone());
        }
        self.terms.push(term)
    }

    /// The terms being described, each a list of inlines
    pub fn terms(&self) -> Vec<Vec<Inline>> {
        self.terms.clone()
    }

    /// The description's text
    pub fn principal(&self) -> Vec<Inline> {
        self.principal.clone()
    }

    pub(crate) fn add_inline(&mut self, inline: Inline) {
        self.principal.push(inline)
    }

    /// Where the item starts and ends in the source
    pub fn locations(&self) -> Vec<Location> {
        self.location.clone()
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
