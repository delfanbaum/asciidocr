use chrono::Local;
use log::warn;
use minijinja::{AutoEscape, Environment, context};

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

// Asciidoctor-style templates
static ASCIIDOCTOR_TEMPLATE: (&str, &str) = (
    "asciidoctor.html",
    include_str!("../../templates/asciidoctor/asciidoctor.html.jinja"),
);
static ASCIIDOCTOR_EMBED_TEMPLATE: (&str, &str) = (
    "asciidoctor.html", // same name, so it can be rendered as the "base" template either way
    include_str!("../../templates/asciidoctor/embed.html.jinja"),
);
static ASCIIDOCTOR_BLOCKS_TEMPLATE: (&str, &str) = (
    "block.html",
    include_str!("../../templates/asciidoctor/block.html.jinja"),
);
static ASCIIDOCTOR_INLINES_TEMPLATE: (&str, &str) = (
    "inline.html",
    include_str!("../../templates/asciidoctor/inline.html.jinja"),
);

/// Renders HTMLBook, which is HTML5 compliant, and includes no styles and no extraneous `<div>`
/// enclosure. Useful when you just want a "plain" HTML representation.
pub fn render_htmlbook(graph: &Asg) -> Result<String, ConversionError> {
    let templates = gather_htmlbook_templates(true);
    render_from_templates(graph, templates, HTMLBOOK_TEMPLATE.0)
}

/// Renders only the contents of the document body as HTMLBook, i.e., without the `<html>`,
/// `<head>` and `<body>` elements, for embedding in another page.
pub fn render_htmlbook_embedded(graph: &Asg) -> Result<String, ConversionError> {
    let templates = gather_htmlbook_templates(false);
    render_from_templates(graph, templates, HTMLBOOK_EMBED_TEMPLATE.0)
}

/// Renders Asciidoctor-styled HTML: the same classes and structure that Asciidoctor produces, so
/// that Asciidoctor's stylesheets apply. Note that this does not include any stylesheet itself.
pub fn render_asciidoctor_html(graph: &Asg) -> Result<String, ConversionError> {
    let templates = gather_asciidoctor_templates(true);
    render_from_templates(graph, templates, ASCIIDOCTOR_TEMPLATE.0)
}

/// Renders only the document contents as Asciidoctor-styled HTML, like running Asciidoctor with
/// its `-s` (standalone off) option, for embedding in another page.
pub fn render_asciidoctor_html_embedded(graph: &Asg) -> Result<String, ConversionError> {
    let templates = gather_asciidoctor_templates(false);
    render_from_templates(graph, templates, ASCIIDOCTOR_EMBED_TEMPLATE.0)
}

/// Renders the template named `base_template` after loading all of `templates` into a fresh
/// environment; each backend gets its own environment, so template names can repeat across them.
fn render_from_templates(
    graph: &Asg,
    templates: Vec<(&'static str, &'static str)>,
    base_template: &str,
) -> Result<String, ConversionError> {
    // Content is already entity-encoded by the parser, so don't escape a second time.
    let mut env = Environment::new();
    env.set_auto_escape_callback(|_| AutoEscape::None);
    // lets templates report recoverable problems in the document (e.g., unsupported titles)
    env.add_function("warn", |message: String| {
        warn!("{message}");
        String::new()
    });
    for (name, template) in templates {
        env.add_template(name, template)
            .unwrap_or_else(|_| panic!("Error loading template: {}", name))
    }
    // set up context
    let build_ctx =
        context!(current_time => Local::now().format("%Y-%m-%d %H:%M:%S %z").to_string());
    let tmpl = env
        .get_template(base_template)
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
    vec![base_template, INLINES_TEMPLATE, BLOCKS_TEMPLATE]
}

fn gather_asciidoctor_templates(full_document: bool) -> Vec<(&'static str, &'static str)> {
    let base_template = if full_document {
        ASCIIDOCTOR_TEMPLATE
    } else {
        ASCIIDOCTOR_EMBED_TEMPLATE
    };
    vec![
        base_template,
        ASCIIDOCTOR_INLINES_TEMPLATE,
        ASCIIDOCTOR_BLOCKS_TEMPLATE,
    ]
}
