use anyhow::Result;
use clap::Parser;
use std::{fs, io, path::PathBuf};

use asciidocr::{cli::Cli, output::render, parser::Parser as AdocParser, scanner::Scanner};

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
            if args.adapter {
                out_destination.set_extension("json")
            } else {
                out_destination.set_extension("html")
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
    let graph = AdocParser::new().parse(Scanner::new(&open(&args.file)));
    if args.adapter {
        Ok(serde_json::to_string_pretty(&graph)?)
    } else {
        render(&graph)
    }
}

fn open(filename: &str) -> String {
    match filename {
        "-" => io::read_to_string(io::stdin()).expect("Error reading from stdin"),
        _ => fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to read file: {}", filename)),
    }
}

fn output(result: String, destination: Option<PathBuf>) {
    match destination {
        Some(out_file) => fs::write(out_file, result).expect("Error writng file"),
        None => println!("{}", result),
    }
}
