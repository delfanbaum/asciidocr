//! What the HTML backends share: rendering their templates, escaping and section IDs

use chrono::Local;
use log::warn;
use minijinja::{AutoEscape, Environment, Value, context, value::Serde};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

use crate::errors::ConversionError;
use crate::graph::asg::Asg;
use crate::graph::blocks::Block;

/// Escapes the characters that would otherwise be read as markup when placed in HTML text.
/// Quotation marks are left alone, as in Asciidoctor's output. Like Asciidoctor, an ampersand that
/// already starts a character reference (`&amp;`, `&#8217;`, `&#x2014;`) is kept as it is: the
/// parser makes character references of replacements and of references the author wrote, but
/// inside spans they can be merged into the surrounding text.
fn escape_text(text: &str) -> String {
    static CHARREF: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"&((?:[a-zA-Z][a-zA-Z]+\d{0,2}|#\d\d\d{0,4}|#[xX][\da-fA-F][\da-fA-F][\da-fA-F]{0,3});)?")
            .unwrap()
    });
    let text = CHARREF.replace_all(text, |caps: &regex::Captures| match caps.get(1) {
        Some(reference) => format!("&{}", reference.as_str()),
        None => "&amp;".to_string(),
    });
    text.replace('<', "&lt;").replace('>', "&gt;")
}

/// Escapes text for use inside a double-quoted HTML attribute value.
fn escape_attr(text: &str) -> String {
    escape_text(text).replace('"', "&quot;")
}

/// Where a block starts, as a key to find it by
fn location_key(line: usize, col: usize, files: &[String]) -> String {
    format!("{line}:{col}:{}", files.join("\u{0}"))
}

/// Every section's ID (the one it was given, or the one the parser generated), by where the
/// section starts
fn section_ids_by_location(graph: &Asg) -> HashMap<String, String> {
    fn collect(block: &Block, ids: &mut HashMap<String, String>) {
        if let (Some(id), Some(start)) = (block.section_id(), block.locations().first()) {
            ids.insert(location_key(start.line, start.col, &start.file), id);
        }
        for child in block.child_blocks() {
            collect(child, ids);
        }
    }
    let mut ids = HashMap::new();
    for block in &graph.blocks {
        collect(block, &mut ids);
    }
    ids
}

/// The `location_key` of a block, as the templates see it
fn location_key_of(block: &Value) -> Option<String> {
    let start = block
        .get_attr("location")
        .ok()?
        .get_item(&Value::from(0))
        .ok()?;
    let line = start.get_attr("line").ok()?.as_usize()?;
    let col = start.get_attr("col").ok()?.as_usize()?;
    let files: Vec<String> = start
        .get_attr("file")
        .ok()
        .and_then(|file| file.try_iter().ok())
        .map(|files| files.map(|file| file.to_string()).collect())
        .unwrap_or_default();
    Some(location_key(line, col, &files))
}

/// Renders the template named `base_template` after loading all of `templates` into a fresh
/// environment; each backend gets its own environment, so template names can repeat across them.
/// `add_helpers` registers the functions only that backend's templates use.
pub(super) fn render(
    graph: &Asg,
    templates: &[(&'static str, &'static str)],
    base_template: &str,
    add_helpers: impl FnOnce(&mut Environment<'static>),
) -> Result<String, ConversionError> {
    // Content is already entity-encoded by the parser, so don't escape a second time.
    let mut env = Environment::new();
    env.set_auto_escape_callback(|_| AutoEscape::None);
    // lets templates report recoverable problems in the document (e.g., unsupported titles)
    env.add_function("warn", |message: String| {
        warn!("{message}");
        String::new()
    });
    // a section's ID: the one it was given, or the one the parser generated (see
    // `crate::graph::ids`); none if `sectids` is unset
    let section_ids = section_ids_by_location(graph);
    env.add_function("section_id", move |section: Value| -> Option<String> {
        section_ids.get(&location_key_of(&section)?).cloned()
    });
    // The parser leaves special characters in text as they were written (and makes character
    // references of replacements, which must not be escaped again), so templates escape text and
    // attribute values themselves
    env.add_filter("escape_text", |text: String| escape_text(&text));
    env.add_filter("escape_attr", |text: String| escape_attr(&text));
    // what the backend's own templates need
    add_helpers(&mut env);
    for &(name, template) in templates {
        env.add_template(name, template)
            .unwrap_or_else(|_| panic!("Error loading template: {}", name))
    }
    // set up context
    let build_ctx =
        context!(current_time => Local::now().format("%Y-%m-%d %H:%M:%S %z").to_string());
    let tmpl = env
        .get_template(base_template)
        .expect("Error finding base template.");

    // do the rendering; since minijinja 3, serde types have to be wrapped in `Serde` explicitly
    // the revision line isn't part of the graph's serialized form (the ASG has no place for it)
    let revision = graph
        .header
        .as_ref()
        .and_then(|header| header.revision.clone());
    Ok(tmpl.render(context!(graph => Serde(graph), revision => Serde(revision), ..build_ctx))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::all_three("a < b & c > d", "a &lt; b &amp; c &gt; d")]
    #[case::quotes_untouched("say \"hi\"", "say \"hi\"")]
    #[case::character_references_kept(
        "a &amp; b &#8217; c &#x2014; d &lt;",
        "a &amp; b &#8217; c &#x2014; d &lt;"
    )]
    #[case::not_a_reference("AT&T; and &x;", "AT&amp;T; and &amp;x;")]
    fn test_escape_text(#[case] text: &str, #[case] expected: &str) {
        assert_eq!(escape_text(text), expected);
    }

    #[test]
    fn test_escape_attr_escapes_quotes_too() {
        assert_eq!(
            escape_attr("a \"b\" & <c>"),
            "a &quot;b&quot; &amp; &lt;c&gt;"
        );
    }
}
