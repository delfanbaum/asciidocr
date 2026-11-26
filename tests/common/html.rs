//! Compares HTML the way it renders, so that tests see whitespace that shows (a missing space next
//! to a `<strong>`, a stray newline at the start of a link, anything inside `<pre>`) and ignore
//! whitespace that doesn't (indentation between block elements).
//!
//! Both sides are parsed with an HTML5 parser (`scraper`, on `html5ever`) and printed in a
//! canonical form, one node per line, which [`assert_html_eq`] compares with a line diff:
//!
//! - elements as `<tag a="1" b="2">`, attributes sorted by name (their order doesn't matter in
//!   HTML), with `class` values as written (we copy Asciidoctor's order);
//! - text quoted, with escapes (`"a\nb"`), so whitespace is visible;
//! - the doctype, if any; comments are dropped.
//!
//! Whitespace is normalized as a browser renders it (`white-space: normal`):
//!
//! - in `<pre>`, `<textarea>`, `<script>` and `<style>`, text is exact;
//! - elsewhere, runs of whitespace are one space;
//! - text loses the whitespace it has next to a block boundary (the start or end of a block-level
//!   parent, or a block-level sibling), and text that is only whitespace there goes away;
//! - whitespace next to inline elements is kept, so `a <b>b</b> c` and `a<b>b</b>c` differ.
//!
//! This is per parent, so `<em>a </em> b` (two spaces that a browser would collapse into one)
//! differs from `<em>a</em> b`; such markup differences are worth seeing anyway.

use scraper::{ElementRef, Html, Node, Selector};

/// Elements that start a new block (where surrounding whitespace doesn't render)
const BLOCK_ELEMENTS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "body",
    "br",
    "caption",
    "col",
    "colgroup",
    "dd",
    "details",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "li",
    "link",
    "main",
    "meta",
    "nav",
    "ol",
    "p",
    "pre",
    "script",
    "section",
    "style",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "ul",
];

/// Elements whose text is compared exactly
const VERBATIM_ELEMENTS: &[&str] = &["pre", "textarea", "script", "style"];

fn is_block(name: &str) -> bool {
    BLOCK_ELEMENTS.contains(&name)
}

#[derive(Debug, PartialEq)]
enum Canonical {
    Element {
        name: String,
        attrs: Vec<(String, String)>,
        children: Vec<Canonical>,
    },
    Text(String),
}

impl Canonical {
    fn is_block(&self) -> bool {
        matches!(self, Canonical::Element { name, .. } if is_block(name))
    }

    fn print(&self, depth: usize, out: &mut String) {
        let indent = "  ".repeat(depth);
        match self {
            Canonical::Text(text) => out.push_str(&format!("{indent}{text:?}\n")),
            Canonical::Element {
                name,
                attrs,
                children,
            } => {
                out.push_str(&format!("{indent}<{name}"));
                for (attr, value) in attrs {
                    out.push_str(&format!(" {attr}={value:?}"));
                }
                out.push_str(">\n");
                for child in children {
                    child.print(depth + 1, out);
                }
            }
        }
    }
}

/// The children of an element, normalized; `verbatim` keeps their text exactly as it is
fn canonical_children(element: ElementRef, verbatim: bool) -> Vec<Canonical> {
    let mut children: Vec<Canonical> = vec![];
    for child in element.children() {
        match child.value() {
            Node::Text(text) => match children.last_mut() {
                Some(Canonical::Text(previous)) => previous.push_str(text),
                _ => children.push(Canonical::Text(text.to_string())),
            },
            Node::Element(_) => {
                if let Some(child_element) = ElementRef::wrap(child) {
                    children.push(canonical_element(child_element, verbatim));
                }
            }
            // comments, processing instructions
            _ => {}
        }
    }
    if verbatim {
        return children;
    }
    let parent_is_block = is_block(element.value().name());
    let count = children.len();
    let blocks: Vec<bool> = children.iter().map(Canonical::is_block).collect();
    children
        .into_iter()
        .enumerate()
        .filter_map(|(idx, child)| {
            let Canonical::Text(text) = child else {
                return Some(child);
            };
            let mut text = collapse_whitespace(&text);
            let block_before = if idx == 0 {
                parent_is_block
            } else {
                blocks[idx - 1]
            };
            let block_after = if idx + 1 == count {
                parent_is_block
            } else {
                blocks[idx + 1]
            };
            if block_before {
                text = text.trim_start().to_string();
            }
            if block_after {
                text = text.trim_end().to_string();
            }
            (!text.is_empty()).then_some(Canonical::Text(text))
        })
        .collect()
}

fn collapse_whitespace(text: &str) -> String {
    let mut collapsed = String::with_capacity(text.len());
    let mut in_whitespace = false;
    for c in text.chars() {
        // (HTML's whitespace is ASCII; a no-break space, for one, is text)
        if c.is_ascii_whitespace() {
            if !in_whitespace {
                collapsed.push(' ');
            }
            in_whitespace = true;
        } else {
            collapsed.push(c);
            in_whitespace = false;
        }
    }
    collapsed
}

fn canonical_element(element: ElementRef, verbatim: bool) -> Canonical {
    let name = element.value().name().to_string();
    let mut attrs: Vec<(String, String)> = element
        .value()
        .attrs()
        .map(|(attr, value)| (attr.to_string(), value.to_string()))
        .collect();
    attrs.sort();
    let verbatim = verbatim || VERBATIM_ELEMENTS.contains(&name.as_str());
    Canonical::Element {
        children: canonical_children(element, verbatim),
        name,
        attrs,
    }
}

/// Whether the HTML is a whole document (rather than a fragment, like embedded output)
fn is_document(html: &str) -> bool {
    let start = html.trim_start().to_ascii_lowercase();
    start.starts_with("<!doctype") || start.starts_with("<html")
}

fn parse(html: &str) -> Html {
    if is_document(html) {
        Html::parse_document(html)
    } else {
        Html::parse_fragment(html)
    }
}

/// The canonical form of the HTML (a document, or a fragment such as embedded output), one node
/// per line; see the module docs
pub fn canonical_html(html: &str) -> String {
    let parsed = parse(html);
    let mut out = String::new();
    if is_document(html) {
        for node in parsed.tree.root().children() {
            if let Node::Doctype(doctype) = node.value() {
                out.push_str(&format!("<!DOCTYPE {}>\n", doctype.name()));
            }
        }
        canonical_element(parsed.root_element(), false).print(0, &mut out);
    } else {
        // a fragment is parsed into an `<html>` element of its own; what we want is inside it
        for child in canonical_children(parsed.root_element(), false) {
            child.print(0, &mut out);
        }
    }
    out
}

/// Asserts that two pieces of HTML (documents, or fragments) render the same, with a line diff of
/// their canonical forms if they don't; see the module docs
#[track_caller]
pub fn assert_html_eq(actual: &str, expected: &str) {
    pretty_assertions::assert_eq!(canonical_html(actual), canonical_html(expected));
}

/// The HTML of the elements that match a CSS selector, one after another (separated by a
/// newline), for tests that check part of a page:
/// `assert_html_eq(&select(&html, "dl dl"), "<dl>...</dl>")`. Select whole tables rather than rows
/// or cells: HTML parsing drops a `<tr>` or `<td>` outside a table.
#[track_caller]
pub fn select(html: &str, css: &str) -> String {
    let selector = Selector::parse(css).unwrap_or_else(|e| panic!("Bad selector {css:?}: {e}"));
    parse(html)
        .select(&selector)
        .map(|element| element.html())
        .collect::<Vec<_>>()
        .join("\n")
}
