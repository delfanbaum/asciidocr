use chrono::Local;
use log::warn;
use minijinja::{AutoEscape, Environment, context};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

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

/// Builds section ids the way Asciidoctor does when none is given: `_` followed by the lowercased
/// title text, with markup and punctuation removed and spaces, periods and hyphens turned into
/// single underscores (e.g., "An A-Head" becomes `_an_a_head`).
fn asciidoctor_section_id(title_html: &str) -> String {
    static INVALID_CHARS: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"<[^>]+>|&(?:[a-z][a-z]+\d{0,2}|#\d{3,6}|#x[\da-f]{3,5});|[^ \w\-.]+").unwrap()
    });
    static SEPARATORS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ .\-]+").unwrap());
    let lowered = title_html.to_lowercase();
    let cleaned = INVALID_CHARS.replace_all(&lowered, "");
    let id = SEPARATORS.replace_all(&cleaned, "_");
    format!("_{}", id.trim_end_matches('_'))
}

/// Asciidoctor's fallback alt text for an image: the file name without folders or extension,
/// with spaces in place of hyphens and underscores (e.g., `img/my-pic_1.png` becomes "my pic 1").
fn asciidoctor_image_alt(target: &str) -> String {
    let file_name = target.rsplit('/').next().unwrap_or(target);
    let stem = file_name.split('.').next().unwrap_or(file_name);
    stem.replace(['-', '_'], " ")
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
    // Asciidoctor-style templates generate ids for sections that lack one; ids must be unique, so
    // repeats get a numeric suffix (e.g., `_title_2`)
    let seen_ids: Mutex<HashMap<String, usize>> = Mutex::new(HashMap::new());
    env.add_function("section_id", move |title_html: String| {
        let id = asciidoctor_section_id(&title_html);
        let mut seen = seen_ids.lock().unwrap();
        let count = seen.entry(id.clone()).or_insert(0);
        *count += 1;
        if *count == 1 {
            id
        } else {
            format!("{id}_{count}")
        }
    });
    // Asciidoctor numbers captioned blocks by kind ("Figure 1.", "Figure 2.", ...); the templates
    // ask for the next number as they go
    let counters: Mutex<HashMap<String, usize>> = Mutex::new(HashMap::new());
    env.add_function("next_number", move |kind: String| {
        let mut counters = counters.lock().unwrap();
        let count = counters.entry(kind).or_insert(0);
        *count += 1;
        *count
    });
    // Footnotes are numbered in the order they appear, and listed together at the end of the
    // document, so the templates register each one as they render it
    let footnotes: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let registered_footnotes = Arc::clone(&footnotes);
    env.add_function("register_footnote", move |text_html: String| {
        let mut notes = registered_footnotes.lock().unwrap();
        notes.push(text_html);
        notes.len()
    });
    env.add_function("footnotes", move || footnotes.lock().unwrap().clone());
    env.add_function("image_alt", |target: String| asciidoctor_image_alt(&target));
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::simple("An A-Head", "_an_a_head")]
    #[case::numbers_and_periods("Section 1.2 Overview", "_section_1_2_overview")]
    #[case::markup("A <em>marked</em> up title", "_a_marked_up_title")]
    #[case::entities_and_punctuation("Q&amp;A: what's next?", "_qa_whats_next")]
    // Asciidoctor replaces " -- " with a thin space, an em dash and a thin space before making
    // the id, then drops the resulting entities
    #[case::replaced_punctuation("One&#8201;&#8212;&#8201;two", "_onetwo")]
    fn test_asciidoctor_section_id(#[case] title: &str, #[case] expected: &str) {
        assert_eq!(asciidoctor_section_id(title), expected);
    }

    #[rstest]
    #[case::plain("pixel.png", "pixel")]
    #[case::separators("my-other_pixel.png", "my other pixel")]
    #[case::folders("img/sub/pic.jpg", "pic")]
    fn test_asciidoctor_image_alt(#[case] target: &str, #[case] expected: &str) {
        assert_eq!(asciidoctor_image_alt(target), expected);
    }
}
