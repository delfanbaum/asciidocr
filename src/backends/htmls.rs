use tera::{Context, Tera};

use crate::errors::ConversionError;
use crate::graph::asg::Asg;

// HTMLBook templates
static HTMLBOOK_TEMPLATE: &str = include_str!("../../templates/htmlbook/htmlbook.html.tera");
static HTMLBOOK_EMBED_TEMPLATE: &str = include_str!("../../templates/htmlbook/embed.html.tera");
static BLOCKS_TEMPLATE: &str = include_str!("../../templates/htmlbook/block.html.tera");
static LEAF_BLOCKS_TEMPLATE: &str = include_str!("../../templates/htmlbook/leafblocks.html.tera");
static TABLES_TEMPLATE: &str = include_str!("../../templates/htmlbook/tables.html.tera");
static INLINES_TEMPLATE: &str = include_str!("../../templates/htmlbook/inline.html.tera");

// Asciidoctor-compatible templates
static ASC_DR_HTML_TEMPLATE: &str =
    include_str!("../../templates/asciidoctor/asciidoctor.html.tera");
static ASC_DR_BLOCKS_TEMPLATE: &str = include_str!("../../templates/asciidoctor/block.html.tera");
static ASC_DR_LEAF_BLOCKS_TEMPLATE: &str =
    include_str!("../../templates/asciidoctor/leafblocks.html.tera");
static ASC_DR_TABLES_TEMPLATE: &str = include_str!("../../templates/asciidoctor/tables.html.tera");
static ASC_DR_INLINES_TEMPLATE: &str = include_str!("../../templates/asciidoctor/inline.html.tera");

/// Renders HTMLBook, which is HTML5 compliant, and includes no styles and no extraneous `<div>`
/// enclosure. Useful when you just want a "plain" HTML representation.
pub fn render_htmlbook(graph: &Asg) -> Result<String, ConversionError> {
    let (base_template, templates) = gather_htmlbook_templates(true);
    render_from_templates(graph, base_template, templates)
}

/// Renders HTMLBook, which is HTML5 compliant, and includes no styles and no extraneous `<div>`
/// enclosure. Useful when you just want a "plain" HTML representation.
pub fn render_htmlbook_embedded(graph: &Asg) -> Result<String, ConversionError> {
    let (base_template, templates) = gather_htmlbook_templates(false);
    render_from_templates(graph, base_template, templates)
}

/// Renders Asciidoctor-style HTML, which is HTML5 compliant, and includes css selectors, div
/// wraps, etc., that are tracked by the asciidoctor CSS styles, which are *not* included in the
/// output.
pub fn render_asciidoctor_html(graph: &Asg) -> Result<String, ConversionError> {
    let (base_template, templates) = gather_asciidoctor_templates();
    render_from_templates(graph, base_template, templates)
}

fn render_from_templates(
    graph: &Asg,
    base_template: &'static str,
    templates: Vec<(&'static str, &'static str)>,
) -> Result<String, ConversionError> {
    // from their docs
    let mut tera = Tera::default();
    tera.add_raw_templates(templates).expect("failure");
    Ok(tera
        .render(base_template, &Context::from_serialize(graph)?)
        .expect("failure"))
}

fn gather_htmlbook_templates(
    full_document: bool,
) -> (&'static str, Vec<(&'static str, &'static str)>) {
    let mut base_template = "htmlbook.html.tera";
    if !full_document {
        base_template = "embed.html.tera";
    };
    (
        base_template,
        vec![
            ("htmlbook.html.tera", HTMLBOOK_TEMPLATE),
            ("embed.html.tera", HTMLBOOK_EMBED_TEMPLATE),
            ("block.html.tera", BLOCKS_TEMPLATE),
            ("leafblocks.html.tera", LEAF_BLOCKS_TEMPLATE),
            ("tables.html.tera", TABLES_TEMPLATE),
            ("inline.html.tera", INLINES_TEMPLATE),
        ],
    )
}

fn gather_asciidoctor_templates() -> (&'static str, Vec<(&'static str, &'static str)>) {
    (
        "asciidoctor.html.tera",
        vec![
            ("asciidoctor.html.tera", ASC_DR_HTML_TEMPLATE),
            ("block.html.tera", ASC_DR_BLOCKS_TEMPLATE),
            ("leafblocks.html.tera", ASC_DR_LEAF_BLOCKS_TEMPLATE),
            ("tables.html.tera", ASC_DR_TABLES_TEMPLATE),
            ("inline.html.tera", ASC_DR_INLINES_TEMPLATE),
        ],
    )
}
