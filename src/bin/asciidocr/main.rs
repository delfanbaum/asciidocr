mod cli;

use anyhow::Result;
use clap::Parser;
use simple_logger::SimpleLogger;
use std::{fs, path::PathBuf};

use asciidocr::{
    backends::{Backends, htmls::render_htmlbook},
    parser::Parser as AdocParser,
    scanner::Scanner,
};

#[cfg(feature = "docx")]
use asciidocr::backends::docx::render_docx;

use cli::{Cli, read_input, read_output};

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
    let graph = match args.do_not_resolve_targets {
        true => AdocParser::new_no_target_resolution(PathBuf::from(&args.file))
            .parse(Scanner::new(&read_input(&args)))?,
        false => {
            AdocParser::new(PathBuf::from(&args.file)).parse(Scanner::new(&read_input(&args)))?
        }
    };
    if args.count {
        println!("{} words in {}", graph.word_count(), args.file)
    }

    match args.backend {
        Backends::Htmlbook => {
            render_string(render_htmlbook(&graph)?, read_output(args));
            Ok(())
        }

        Backends::Asciidoctor => todo!(),

        Backends::Json => {
            render_string(serde_json::to_string_pretty(&graph)?, read_output(args));
            Ok(())
        }

        #[cfg(feature = "docx")]
        Backends::Docx => {
            if args.do_not_resolve_targets {
                eprintln!(
                    "Error: the docx backend does not support parsing without target resolution."
                );
                std::process::exit(1)
            }

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
