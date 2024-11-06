use anyhow::Result;
use tera::{Context, Tera};

use crate::asg::Asg;

static HTMLBOOK_TEMPLATE: &'static str = include_str!("../templates/htmlbook.html.tera");
static BLOCKS_TEMPLATE: &'static str = include_str!("../templates/blocks/block.html.tera");
static INLINES_TEMPLATE: &'static str = include_str!("../templates/inlines/inline.html.tera");

pub fn render(graph: &Asg) -> Result<String> {
    // from their docs
    let mut tera  = Tera::default();
    tera.add_raw_templates(gather_htmlbook_templates())?;
    Ok(tera.render("htmlbook.html.tera", &Context::from_serialize(graph)?)?)
}

pub fn gather_htmlbook_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("htmlbook.html.tera", HTMLBOOK_TEMPLATE),
        ("blocks/block.html.tera", BLOCKS_TEMPLATE),
        ("inlines/inline.html.tera", INLINES_TEMPLATE),
    ]
}
