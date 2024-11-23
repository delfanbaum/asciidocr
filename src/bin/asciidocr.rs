use anyhow::Result;
use clap::Parser;
use simple_logger::SimpleLogger;
use std::{fs, path::PathBuf};

use asciidocr::{
    backends::docx::render_docx,
    backends::htmls::{gather_htmlbook_templates, render_from_templates},
    cli::{read_output, Backends, Cli},
    parser::Parser as AdocParser,
    scanner::Scanner,
    utils::open_file,
};

fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Warn).with_colors(true).with_timestamps(false).init().unwrap();
    let args = Cli::parse();

    if let Err(e) = run(args) {
        eprintln!("Error converting document: {}", e);
        std::process::exit(1)
    }
}

fn run(args: Cli) -> Result<()> {
    let graph = AdocParser::new().parse(Scanner::new(&open_file(&args.file)));
    match args.backend {
        Backends::Htmlbook => {
            render_string(
                render_from_templates(&graph, gather_htmlbook_templates())?,
                read_output(args),
            );
            Ok(())
        }
        Backends::Json => {
            render_string(serde_json::to_string_pretty(&graph)?, read_output(args));
            Ok(())
        }

        Backends::Docx => {
            if let Some(output_path) = read_output(args) {
                render_docx(&graph, &output_path).expect("Error rendering docx");
                Ok(())
            } else {
                eprintln!("Error: can't send docx backend to stdout");
                std::process::exit(1)
            }
        }
    }
}

fn render_string(result: String, output_destination: Option<PathBuf>) {
    match output_destination {
        Some(out_file) => fs::write(out_file, result).expect("Error writng file"),
        None => println!("{}", result),
    }
}
