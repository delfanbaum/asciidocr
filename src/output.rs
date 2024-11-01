use anyhow::Result;
use tera::{Context, Tera};

use crate::asg::Asg;

pub fn render(graph: &Asg) -> Result<String> {
    // from their docs
    let tera = match Tera::new("templates/**/*.html.tera") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };
    Ok(tera.render("htmlbook.html.tera", &Context::from_serialize(graph)?)?)
}
