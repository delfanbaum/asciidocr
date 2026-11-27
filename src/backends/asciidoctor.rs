//! The Asciidoctor-style HTML backend

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use minijinja::Environment;

use crate::errors::ConversionError;
use crate::graph::asg::Asg;
use crate::graph::lists::NumeralStyle;

use super::html;

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

/// Renders Asciidoctor-styled HTML: the same classes and structure that Asciidoctor produces, so
/// that Asciidoctor's stylesheets apply. Note that this does not include any stylesheet itself.
pub fn render_asciidoctor_html(graph: &Asg) -> Result<String, ConversionError> {
    let templates = gather_asciidoctor_templates(true);
    html::render(graph, &templates, ASCIIDOCTOR_TEMPLATE.0, add_helpers)
}

/// Renders only the document contents as Asciidoctor-styled HTML, like running Asciidoctor with
/// its `-s` (standalone off) option, for embedding in another page.
pub fn render_asciidoctor_html_embedded(graph: &Asg) -> Result<String, ConversionError> {
    let templates = gather_asciidoctor_templates(false);
    html::render(graph, &templates, ASCIIDOCTOR_EMBED_TEMPLATE.0, add_helpers)
}

/// The functions only these templates use
fn add_helpers(env: &mut Environment<'static>) {
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
    // ordered lists are numbered by their marker (`a.` is `loweralpha`, ...)
    env.add_function("numeral_style", |marker: String| {
        NumeralStyle::from_marker(&marker).name()
    });
}

/// Asciidoctor's fallback alt text for an image: the file name without folders or (only the
/// last) extension, with spaces in place of hyphens and underscores (e.g., `img/my-pic_1.png`
/// becomes "my pic 1", and `my.pic.v2.png` becomes "my.pic.v2").
fn asciidoctor_image_alt(target: &str) -> String {
    let file_name = target.rsplit('/').next().unwrap_or(target);
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    stem.replace(['-', '_'], " ")
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
    #[case::plain("pixel.png", "pixel")]
    #[case::separators("my-other_pixel.png", "my other pixel")]
    #[case::folders("img/sub/pic.jpg", "pic")]
    #[case::dots_in_name("img/my.pic.v2.png", "my.pic.v2")]
    #[case::no_extension("noext", "noext")]
    fn test_asciidoctor_image_alt(#[case] target: &str, #[case] expected: &str) {
        assert_eq!(asciidoctor_image_alt(target), expected);
    }
}
