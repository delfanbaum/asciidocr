//! The HTMLBook backend (<https://oreillymedia.github.io/HTMLBook/>)

use crate::errors::ConversionError;
use crate::graph::asg::Asg;

use super::html;

static HTMLBOOK_TEMPLATE: (&str, &str) = (
    "htmlbook.html",
    include_str!("../../templates/htmlbook/htmlbook.html.jinja"),
);
static HTMLBOOK_EMBED_TEMPLATE: (&str, &str) = (
    "htmlbook.html", // still call it "htmlbook" for easy "base" template reference
    include_str!("../../templates/htmlbook/embed.html.jinja"),
);
static BLOCKS_TEMPLATE: (&str, &str) = (
    "block.html",
    include_str!("../../templates/htmlbook/block.html.jinja"),
);
static INLINES_TEMPLATE: (&str, &str) = (
    "inline.html",
    include_str!("../../templates/htmlbook/inline.html.jinja"),
);

/// Renders HTMLBook, which is HTML5 compliant, and includes no styles and no extraneous `<div>`
/// enclosure. Useful when you just want a "plain" HTML representation.
pub fn render_htmlbook(graph: &Asg) -> Result<String, ConversionError> {
    let templates = gather_htmlbook_templates(true);
    html::render(graph, &templates, HTMLBOOK_TEMPLATE.0, |_| {})
}

/// Renders only the contents of the document body as HTMLBook, i.e., without the `<html>`,
/// `<head>` and `<body>` elements, for embedding in another page.
pub fn render_htmlbook_embedded(graph: &Asg) -> Result<String, ConversionError> {
    let templates = gather_htmlbook_templates(false);
    html::render(graph, &templates, HTMLBOOK_EMBED_TEMPLATE.0, |_| {})
}

fn gather_htmlbook_templates(full_document: bool) -> Vec<(&'static str, &'static str)> {
    let base_template = if full_document {
        HTMLBOOK_TEMPLATE
    } else {
        HTMLBOOK_EMBED_TEMPLATE
    };
    vec![base_template, INLINES_TEMPLATE, BLOCKS_TEMPLATE]
}
