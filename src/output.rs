use anyhow::Result;
use tera::{Context, Tera};

use crate::asg::Asg;

static HTMLBOOK_TEMPLATE: &str = include_str!("../templates/htmlbook/htmlbook.html.tera");
static BLOCKS_TEMPLATE: &str = include_str!("../templates/htmlbook/block.html.tera");
static INLINES_TEMPLATE: &str = include_str!("../templates/htmlbook/inline.html.tera");

pub fn render_from_templates(
    graph: &Asg,
    templates: Vec<(&'static str, &'static str)>,
) -> Result<String> {
    // from their docs
    let mut tera = Tera::default();
    tera.add_raw_templates(templates)?;
    Ok(tera.render("htmlbook.html.tera", &Context::from_serialize(graph)?).expect("failure"))
}

pub fn gather_htmlbook_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("htmlbook.html.tera", HTMLBOOK_TEMPLATE),
        ("block.html.tera", BLOCKS_TEMPLATE),
        ("inline.html.tera", INLINES_TEMPLATE),
    ]
}
