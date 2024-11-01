use anyhow::Result;
use clap::Parser;
use std::{fs, io};
use tera::{Context, Tera};

use asciidocr::{asg::Asg, cli::Cli, parser::Parser as AdocParser, scanner::Scanner};

fn main() {
    let args = Cli::parse();
    let graph = AdocParser::new().parse(Scanner::new(&open(&args.file)));
    if args.adapter {
        println!(
            "{}",
            serde_json::to_string_pretty(&graph).expect("Failed to parse graph")
        );
    } else {
        match render(&graph) {
            Ok(rendered) => println!("{}", rendered),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}

fn render(graph: &Asg) -> Result<String> {
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

fn open(filename: &str) -> String {
    match filename {
        "-" => io::read_to_string(io::stdin()).expect("Error reading from stdin"),
        _ => fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to read file: {}", filename)),
    }
}
