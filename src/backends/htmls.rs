use tera::{Context, Tera};

use crate::errors::ConversionError;
use crate::graph::asg::Asg;

static HTMLBOOK_TEMPLATE: &str = include_str!("../../templates/htmlbook/htmlbook.html.tera");
static BLOCKS_TEMPLATE: &str = include_str!("../../templates/htmlbook/block.html.tera");
static LEAF_BLOCKS_TEMPLATE: &str = include_str!("../../templates/htmlbook/leafblocks.html.tera");
static TABLES_TEMPLATE: &str = include_str!("../../templates/htmlbook/tables.html.tera");
static INLINES_TEMPLATE: &str = include_str!("../../templates/htmlbook/inline.html.tera");

/// Renders HTMLBook, which is HTML5 compliant, and includes no styles and no extraneous `<div>`
/// enclosure. Useful when you just want a "plain" HTML representation.
pub fn render_htmlbook(graph: &Asg) -> Result<String, ConversionError> {
    render_from_templates(graph, gather_htmlbook_templates())
}

fn render_from_templates(
    graph: &Asg,
    templates: Vec<(&'static str, &'static str)>,
) -> Result<String, ConversionError> {
    // from their docs
    let mut tera = Tera::default();
    tera.add_raw_templates(templates).expect("failure");
    Ok(tera
        .render("htmlbook.html.tera", &Context::from_serialize(graph)?)
        .expect("failure"))
}

fn gather_htmlbook_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("htmlbook.html.tera", HTMLBOOK_TEMPLATE),
        ("block.html.tera", BLOCKS_TEMPLATE),
        ("leafblocks.html.tera", LEAF_BLOCKS_TEMPLATE),
        ("tables.html.tera", TABLES_TEMPLATE),
        ("inline.html.tera", INLINES_TEMPLATE),
    ]
}
