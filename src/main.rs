use anyhow::Result;
use clap::Parser;
use std::{fs, path::PathBuf};

use asciidocr::{
    cli::{Backends, Cli},
    output::{gather_htmlbook_templates, render_from_templates},
    parser::Parser as AdocParser,
    scanner::Scanner,
    utils::open_file,
};

fn main() {
    let args = Cli::parse();
    let destination = match args.output {
        Some(ref output) => {
            if output == "-" {
                None
            } else {
                let mut out_destination = PathBuf::from(args.file.clone());
                out_destination.set_extension("html");
                Some(out_destination)
            }
        }
        None => {
            let mut out_destination = PathBuf::new();
            out_destination.push(args.file.clone());
            match args.backend {
                Backends::Htmlbook => out_destination.set_extension("html"),
                Backends::Json => out_destination.set_extension("json"),
            };
            Some(out_destination)
        }
    };

    match run(args) {
        Ok(result) => output(result, destination),
        Err(e) => {
            eprintln!("Error converting document: {}", e);
            std::process::exit(1)
        }
    }
}

fn run(args: Cli) -> Result<String> {
    let graph = AdocParser::new().parse(Scanner::new(&open_file(&args.file)));
    match args.backend {
        Backends::Htmlbook => render_from_templates(&graph, gather_htmlbook_templates()),
        Backends::Json => Ok(serde_json::to_string_pretty(&graph)?),
    }
}

fn output(result: String, destination: Option<PathBuf>) {
    match destination {
        Some(out_file) => fs::write(out_file, result).expect("Error writng file"),
        None => println!("{}", result),
    }
}
