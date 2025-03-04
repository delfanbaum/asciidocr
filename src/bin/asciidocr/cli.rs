use asciidocr::backends::Backends;
use clap::Parser;
use log::warn;
use std::{fs, io, path::PathBuf};

/// Main entrypoint for asciidocr when called as executable
#[derive(Parser)]
#[command(name = "asciidocr", version, about)]
pub struct Cli {
    /// Asciidoc file for processing. To read from standard input (stdin), use "-".
    pub file: String,

    /// Provide a filename for the output.
    /// To send to standard out (stdout), use "-".
    #[arg(short = 'o', long = "out-file")]
    pub output: Option<String>,

    /// Select a backend for conversion.
    #[arg(value_enum, short = 'b', long = "backend", default_value = "htmlbook")]
    pub backend: Backends,

    /// Produces an embeddable document, which includes only content that would normally fall
    /// inside the `<body>` tags
    #[arg(short = 'e', long = "embedded")]
    pub embedded: bool,

    /// Provide a stylesheet to be embedded inside the resultant document `head` (applies to the
    /// HTML backend only; this flag is ignored when used with other backends)
    #[arg(short = 's', long = "stylesheet")]
    pub embed_stylesheet: Option<String>,
}

pub fn read_input(args: &Cli) -> String {
    match args.file.as_str() {
        "-" => io::read_to_string(io::stdin()).expect("Error reading from stdin"),
        _ => match fs::read_to_string(args.file.as_str()) {
            Ok(file_string) => file_string,
            Err(e) => {
                warn!("Unable to read file {:?}: {e}", &args.file.as_str());
                std::process::exit(1)
            }
        },
    }
}

pub fn read_output(args: Cli) -> Option<PathBuf> {
    match args.output {
        Some(ref output) => {
            if output == "-" {
                None
            } else {
                Some(PathBuf::from(output.clone()))
            }
        }
        None => {
            if args.file == "-" {
                // we put to stdout if stdin following asciidoctor
                None
            } else {
                let mut out_destination = PathBuf::new();
                out_destination.push(args.file.clone());
                match args.backend {
                    Backends::Htmlbook => out_destination.set_extension("html"),
                    Backends::Json => out_destination.set_extension("json"),
                    //#[cfg(feature = "docx")]
                    Backends::Docx => out_destination.set_extension("docx"),
                };
                Some(out_destination)
            }
        }
    }
}
