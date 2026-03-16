use chrono::Local;
use minijinja::{Environment, context};

use crate::errors::ConversionError;
use crate::graph::asg::Asg;

// HTMLBook templates
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
    render_from_templates(graph, templates)
}

/// Renders HTMLBook, which is HTML5 compliant, and includes no styles and no extraneous `<div>`
/// enclosure. Useful when you just want a "plain" HTML representation.
pub fn render_htmlbook_embedded(graph: &Asg) -> Result<String, ConversionError> {
    let templates = gather_htmlbook_templates(false);
    render_from_templates(graph, templates)
}

fn render_from_templates(
    graph: &Asg,
    templates: Vec<(&'static str, &'static str)>,
) -> Result<String, ConversionError> {
    // set up tera
    let mut env = Environment::new();
    for (name, template) in templates {
        env.add_template(name, template)
            .expect(&format!("Error loading template: {}", name))
    }
    // set up context
    let build_ctx = context!(current_time => Local::now());
    let tmpl = env
        .get_template("htmlbook.html")
        .expect("Error finding base template.");

    // do the rendering
    Ok(tmpl.render(context!(..build_ctx, ..context! {graph}))?)
}

fn gather_htmlbook_templates(full_document: bool) -> Vec<(&'static str, &'static str)> {
    let base_template = if full_document {
        HTMLBOOK_TEMPLATE
    } else {
        HTMLBOOK_EMBED_TEMPLATE
    };
    vec![
        base_template,
        INLINES_TEMPLATE,
        BLOCKS_TEMPLATE,
    ]
}
