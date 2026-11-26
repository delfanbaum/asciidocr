pub mod html;

use std::env;
use std::{fs, path::PathBuf};

use asciidocr::{
    backends::htmls::{
        render_asciidoctor_html, render_asciidoctor_html_embedded, render_htmlbook,
        render_htmlbook_embedded,
    },
    parser::Parser,
    scanner::Scanner,
};
use assert_json_diff::assert_json_eq;
use image::RgbImage;
use serde_json::{Value, json};
use tempfile::NamedTempFile;

/// Given a pattern "parent/filename" inside tests/data, assert that the filename.adoc produces
/// the expected abstract syntax graph found in filename.json
pub fn assert_parsed_doc_matches_expected_asg(adoc_fn: &str, asg_json_fn: &str) {
    let test_dir = PathBuf::from("tests/data/");
    let parsed_asg = Parser::new_no_target_resolution(test_dir.join(adoc_fn))
        .parse(Scanner::new(
            &fs::read_to_string(test_dir.join(adoc_fn)).expect("Unable to find adoc"),
        ))
        .expect("Unable to parse input file");
    let asg_as_json = json!(parsed_asg);

    let expected_asg: Value = serde_json::from_str(
        &fs::read_to_string(test_dir.join(asg_json_fn)).expect("Unable to find asg json"),
    )
    .unwrap();
    assert_json_eq!(asg_as_json, expected_asg);
}

/// Parses the Asciidoc source and returns all of the text in the resulting graph (every literal
/// `value`, in document order), for tests that only care that nothing was lost or crashed
pub fn parsed_text(adoc: &str) -> String {
    fn collect(value: &Value, text: &mut String) {
        match value {
            Value::Object(map) => {
                if let Some(Value::String(v)) = map.get("value") {
                    text.push_str(v);
                }
                for (key, child) in map {
                    if key != "value" && key != "location" {
                        collect(child, text);
                    }
                }
            }
            Value::Array(items) => items.iter().for_each(|child| collect(child, text)),
            _ => {}
        }
    }
    let asg = json!(
        Parser::new_no_target_resolution(env::current_dir().unwrap())
            .parse(Scanner::new(adoc))
            .expect("Unable to parse input")
    );
    let mut text = String::new();
    collect(&asg, &mut text);
    text
}

/// Parses the Asciidoc source and returns the names of the document's top-level blocks (e.g.,
/// "paragraph", "section"), for tests of how blocks are arranged rather than of their contents
pub fn top_level_block_names(adoc: &str) -> Vec<String> {
    let asg = json!(
        Parser::new_no_target_resolution(env::current_dir().unwrap())
            .parse(Scanner::new(adoc))
            .expect("Unable to parse input")
    );
    asg["blocks"]
        .as_array()
        .expect("Document has no blocks")
        .iter()
        .map(|block| block["name"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// Parses the Asciidoc source and describes how its blocks nest, e.g.,
/// `section(sidebar(paragraph),section(paragraph))`; top-level blocks are separated by spaces
pub fn block_tree(adoc: &str) -> String {
    fn describe(block: &Value) -> String {
        let name = block["name"].as_str().unwrap_or_default();
        let children: Vec<String> = block["blocks"]
            .as_array()
            .map(|blocks| blocks.iter().map(describe).collect())
            .unwrap_or_default();
        if children.is_empty() {
            name.to_string()
        } else {
            format!("{}({})", name, children.join(","))
        }
    }
    let asg = json!(
        Parser::new_no_target_resolution(env::current_dir().unwrap())
            .parse(Scanner::new(adoc))
            .expect("Unable to parse input")
    );
    asg["blocks"]
        .as_array()
        .expect("Document has no blocks")
        .iter()
        .map(describe)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parses the Asciidoc source and outlines its blocks with their text, descending into lists,
/// e.g., `list(* "one"(paragraph "more"), * "two")`; top-level blocks are separated by spaces.
/// List items show their marker and principal text, description list items their terms and
/// principal text (`"Term":: "def"`, or `"A" "B":: "def"` for two terms), sections their title, and other blocks their name and inline
/// text, if any (as it is in the graph: a listing's text starts with a newline, e.g.). This is
/// for tests of where text and blocks end up, without hand-writing JSON with locations.
pub fn block_outline(adoc: &str) -> String {
    outline(&parsed_asg(adoc))
}

/// Like [`block_outline`], but for a file in tests/data (whose includes resolve from where it is)
pub fn block_outline_from_file(adoc_fn: &str) -> String {
    let path = PathBuf::from("tests/data/").join(adoc_fn);
    let asg = json!(
        Parser::new_no_target_resolution(path.clone())
            .parse(Scanner::new(
                &fs::read_to_string(&path).expect("Unable to find adoc")
            ))
            .expect("Unable to parse input")
    );
    outline(&asg)
}

fn outline(asg: &Value) -> String {
    fn text(inlines: &Value) -> String {
        let mut collected = String::new();
        for inline in inlines.as_array().into_iter().flatten() {
            if let Some(Value::String(value)) = inline.get("value") {
                collected.push_str(value);
            }
            collected.push_str(&text(&inline["inlines"]));
        }
        collected
    }
    fn children(block: &Value, key: &str) -> Vec<String> {
        block[key]
            .as_array()
            .map(|blocks| blocks.iter().map(describe).collect())
            .unwrap_or_default()
    }
    fn describe(block: &Value) -> String {
        let name = block["name"].as_str().unwrap_or_default();
        let mut outline = match name {
            "list" | "dlist" => {
                return format!("{}({})", name, children(block, "items").join(", "));
            }
            "listItem" => format!(
                "{} \"{}\"",
                block["marker"].as_str().unwrap_or_default(),
                text(&block["principal"])
            ),
            "dlistItem" => format!(
                "{}:: \"{}\"",
                block["terms"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|term| format!("\"{}\"", text(term)))
                    .collect::<Vec<_>>()
                    .join(" "),
                text(&block["principal"])
            ),
            // a section's text is its title
            "section" => match text(&block["title"]) {
                title if title.is_empty() => name.to_string(),
                title => format!("{} \"{}\"", name, title),
            },
            _ => match text(&block["inlines"]) {
                inline_text if inline_text.is_empty() => name.to_string(),
                inline_text => format!("{} \"{}\"", name, inline_text),
            },
        };
        let blocks = children(block, "blocks");
        if !blocks.is_empty() {
            outline.push_str(&format!("({})", blocks.join(", ")));
        }
        outline
    }
    asg["blocks"]
        .as_array()
        .expect("Document has no blocks")
        .iter()
        .map(describe)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parses the Asciidoc source into its graph, as JSON
pub fn parsed_asg(adoc: &str) -> Value {
    json!(
        Parser::new_no_target_resolution(env::current_dir().unwrap())
            .parse(Scanner::new(adoc))
            .expect("Unable to parse input")
    )
}

/// Like [`block_outline`], but parses on its own thread and panics if parsing takes longer than
/// a few seconds, so that a parser that loops forever fails the test instead of hanging the run
pub fn block_outline_with_timeout(adoc: &str) -> String {
    let (sender, receiver) = std::sync::mpsc::channel();
    let adoc = adoc.to_string();
    std::thread::spawn(move || {
        let _ = sender.send(block_outline(&adoc));
    });
    receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("Parsing timed out (or panicked)")
}

/// Given an asciidoc string and an expected abstract syntax graph json string, assert that the
/// parser produces the correct json from the asciidoc
pub fn assert_parsed_doc_matches_expected_asg_from_str(adoc_str: &str, asg_json_str: &str) {
    let origin = env::current_dir().unwrap();
    let parsed_asg = json!(
        Parser::new_no_target_resolution(origin)
            .parse(Scanner::new(adoc_str))
            .expect("Unable to parse input file")
    );
    let expected_asg: Value = serde_json::from_str(asg_json_str).unwrap();
    assert_json_eq!(parsed_asg, expected_asg);
}

/// Given a pattern "parent/filename" inside tests/data, assert that the filename.adoc produces the
/// HTMLBook document in filename.html, compared as it renders (see [`html::assert_html_eq`])
pub fn assert_rendered_htmlbook_matches_expected(adoc_fn: &str, html_fn: &str) {
    let test_dir = PathBuf::from("tests/data/");
    let rendered_html = render_htmlbook(
        &Parser::new_no_target_resolution(test_dir.join(adoc_fn))
            .parse(Scanner::new(
                &fs::read_to_string(test_dir.join(adoc_fn)).expect("Unable to find adoc"),
            ))
            .expect("Unable to parse input file"),
    )
    .expect("Unable to render HTML from document");
    let expected_html =
        fs::read_to_string(test_dir.join(html_fn)).expect("Unable to read expected html file");
    html::assert_html_eq(&rendered_html, &expected_html);
}

/// Like `assert_rendered_htmlbook_matches_expected`, but for the embedded (no document wrapper)
/// rendering
pub fn assert_rendered_htmlbook_embedded_matches_expected(adoc_fn: &str, html_fn: &str) {
    let test_dir = PathBuf::from("tests/data/");
    let rendered_html = render_htmlbook_embedded(
        &Parser::new_no_target_resolution(test_dir.join(adoc_fn))
            .parse(Scanner::new(
                &fs::read_to_string(test_dir.join(adoc_fn)).expect("Unable to find adoc"),
            ))
            .expect("Unable to parse input file"),
    )
    .expect("Unable to render HTML from document");
    let expected_html =
        fs::read_to_string(test_dir.join(html_fn)).expect("Unable to read expected html file");
    html::assert_html_eq(&rendered_html, &expected_html);
}

/// Given a pattern "parent/filename" inside tests/data, assert that the filename.adoc produces the
/// Asciidoctor-styled full document found in filename.html, compared as it renders (see
/// [`html::assert_html_eq`]), ignoring the "Last updated" timestamp (`NOW` in the expected html).
pub fn assert_rendered_asciidoctor_html_matches_expected(adoc_fn: &str, html_fn: &str) {
    let test_dir = PathBuf::from("tests/data/");
    let rendered_html = render_asciidoctor_html(
        &Parser::new_no_target_resolution(test_dir.join(adoc_fn))
            .parse(Scanner::new(
                &fs::read_to_string(test_dir.join(adoc_fn)).expect("Unable to find adoc"),
            ))
            .expect("Unable to parse input file"),
    )
    .expect("Unable to render HTML from document");
    let timestamp = regex::Regex::new(r"Last updated [^<]*").unwrap();
    let rendered_html = timestamp
        .replace(&rendered_html, "Last updated NOW")
        .to_string();
    let expected_html =
        fs::read_to_string(test_dir.join(html_fn)).expect("Unable to read expected html file");
    html::assert_html_eq(&rendered_html, &expected_html);
}

/// Renders Asciidoc source to embedded HTMLBook, keeping whitespace as is (for tests of things
/// like `<pre>` contents, where whitespace matters)
pub fn render_htmlbook_embedded_from_str(adoc: &str) -> String {
    render_htmlbook_embedded(
        &Parser::new_no_target_resolution(env::current_dir().unwrap())
            .parse(Scanner::new(adoc))
            .expect("Unable to parse input"),
    )
    .expect("Unable to render HTML from document")
}

/// Renders Asciidoc source to embedded Asciidoctor-styled html, keeping whitespace as is (for
/// tests of things like `<pre>` contents, where whitespace matters)
pub fn render_asciidoctor_embedded_from_str(adoc: &str) -> String {
    render_asciidoctor_html_embedded(
        &Parser::new_no_target_resolution(env::current_dir().unwrap())
            .parse(Scanner::new(adoc))
            .expect("Unable to parse input"),
    )
    .expect("Unable to render HTML from document")
}

/// Given a pattern "parent/filename" inside tests/data, assert that the filename.adoc produces the
/// embedded Asciidoctor-styled html (i.e., the document contents only, like `asciidoctor -s`)
/// found in filename.html, compared as it renders (see [`html::assert_html_eq`]).
pub fn assert_rendered_asciidoctor_embedded_matches_expected(adoc_fn: &str, html_fn: &str) {
    let test_dir = PathBuf::from("tests/data/");
    let rendered_html = render_asciidoctor_html_embedded(
        &Parser::new_no_target_resolution(test_dir.join(adoc_fn))
            .parse(Scanner::new(
                &fs::read_to_string(test_dir.join(adoc_fn)).expect("Unable to find adoc"),
            ))
            .expect("Unable to parse input file"),
    )
    .expect("Unable to render HTML from document");
    let expected_html =
        fs::read_to_string(test_dir.join(html_fn)).expect("Unable to read expected html file");
    html::assert_html_eq(&rendered_html, &expected_html);
}

pub fn generate_temp_image() -> NamedTempFile {
    let img_path = NamedTempFile::with_suffix(".png").unwrap();
    let img = RgbImage::new(32, 32);
    img.save(&img_path.path()).expect("Error creating image");
    img_path
}

/// Renders the given Asciidoc source to docx and returns the text of one XML part (for example
/// `word/settings.xml` or `word/document.xml`) from inside the resulting zip archive.
#[cfg(feature = "docx")]
pub fn rendered_docx_part(adoc: &str, part: &str) -> String {
    use std::io::Read;

    let parsed_asg = Parser::new_no_target_resolution(PathBuf::from("tests/data/docx/inline.adoc"))
        .parse(Scanner::new(adoc))
        .expect("Failed to parse document");
    let temp_docx = NamedTempFile::new().unwrap();
    asciidocr::backends::docx::render_docx(&parsed_asg, temp_docx.path())
        .expect("Failed to render docx");
    let mut archive = zip::ZipArchive::new(fs::File::open(temp_docx.path()).unwrap())
        .expect("Rendered docx is not a valid zip archive");
    let mut xml = String::new();
    archive
        .by_name(part)
        .unwrap_or_else(|_| panic!("{part} not found in docx"))
        .read_to_string(&mut xml)
        .unwrap();
    xml
}

/// A flattened view of one docx run, so tests can compare formatting without caring about XML.
#[cfg(feature = "docx")]
#[derive(Debug, Default, PartialEq, serde::Serialize)]
pub struct RunSummary {
    pub text: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub mono: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub highlight: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vert_align: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub line_break: bool,
}

#[cfg(feature = "docx")]
impl RunSummary {
    fn same_formatting(&self, other: &RunSummary) -> bool {
        !self.line_break
            && !other.line_break
            && (
                self.bold,
                self.italic,
                self.mono,
                self.highlight,
                &self.vert_align,
            ) == (
                other.bold,
                other.italic,
                other.mono,
                other.highlight,
                &other.vert_align,
            )
    }
}

/// Renders Asciidoc source to docx and summarizes the runs of every body paragraph.
#[cfg(feature = "docx")]
pub fn rendered_docx_runs(adoc: &str) -> Vec<Vec<RunSummary>> {
    let xml = rendered_docx_part(adoc, "word/document.xml");
    let doc = roxmltree::Document::parse(&xml).expect("document.xml is not valid XML");
    let named = |n: &roxmltree::Node, name: &str| n.is_element() && n.tag_name().name() == name;
    doc.descendants()
        .filter(|n| named(n, "p"))
        .map(|p| {
            p.children()
                .filter(|r| named(r, "r"))
                .map(|r| {
                    let mut run = RunSummary::default();
                    for prop in r.descendants().filter(|n| n.is_element()) {
                        match prop.tag_name().name() {
                            "b" => run.bold = true,
                            "i" => run.italic = true,
                            "highlight" => run.highlight = true,
                            "rFonts" => {
                                run.mono = prop.attributes().any(|a| a.value() == "Courier New")
                            }
                            "vertAlign" => {
                                run.vert_align =
                                    prop.attributes().next().map(|a| a.value().to_string())
                            }
                            "br" => run.line_break = true,
                            "t" => run.text.push_str(prop.text().unwrap_or_default()),
                            _ => {}
                        }
                    }
                    run
                })
                // adjacent runs with the same formatting look the same, and how the writer splits
                // text into runs (a typographic apostrophe is its own, e.g.) isn't what's tested
                .fold(Vec::<RunSummary>::new(), |mut runs, run| {
                    match runs.last_mut() {
                        Some(last) if last.same_formatting(&run) => last.text.push_str(&run.text),
                        _ => runs.push(run),
                    }
                    runs
                })
        })
        .collect()
}

/// Asserts that the runs rendered from `adoc_fn` (inside tests/data) match the expected JSON.
#[cfg(feature = "docx")]
pub fn assert_rendered_docx_runs_match_expected(adoc_fn: &str, json_fn: &str) {
    let test_dir = PathBuf::from("tests/data/");
    let adoc = fs::read_to_string(test_dir.join(adoc_fn)).expect("Unable to find adoc");
    let expected: Value = serde_json::from_str(
        &fs::read_to_string(test_dir.join(json_fn)).expect("Unable to find expected json"),
    )
    .expect("Expected json is invalid");
    let actual = serde_json::to_value(rendered_docx_runs(&adoc)).unwrap();
    assert_json_eq!(actual, expected);
}
