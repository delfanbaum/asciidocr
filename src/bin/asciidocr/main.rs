mod cli;

use anyhow::Result;
use clap::Parser;
use simple_logger::SimpleLogger;
use std::{fs, path::PathBuf};

use asciidocr::{
    backends::{htmls::render_htmlbook, Backends},
    parser::Parser as AdocParser,
    scanner::Scanner,
};

//#[cfg(feature = "docx")]
use asciidocr::backends::docx::render_docx;

use cli::{read_input, read_output, Cli};

fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .with_colors(true)
        .without_timestamps()
        .init()
        .unwrap();
    let args = Cli::parse();

    if let Err(e) = run(args) {
        eprintln!("Error converting document: {}", e);
        std::process::exit(1)
    }
}

fn run(args: Cli) -> Result<()> {
    let graph = AdocParser::new(PathBuf::from(&args.file)).parse(Scanner::new(&read_input(&args)));
    match args.backend {
        Backends::Htmlbook => {
            render_string(render_htmlbook(&graph)?, read_output(args));
            Ok(())
        }
        Backends::Json => {
            render_string(serde_json::to_string_pretty(&graph)?, read_output(args));
            Ok(())
        }

        //#[cfg(feature = "docx")]
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
