mod cli;

use anyhow::{Context, Result, bail};
use clap::Parser;
use simple_logger::SimpleLogger;
use std::{
    fs,
    path::{Path, PathBuf},
};

use asciidocr::{
    backends::{
        Backends,
        asciidoctor::{render_asciidoctor_html, render_asciidoctor_html_embedded},
        htmlbook::{render_htmlbook, render_htmlbook_embedded},
    },
    parser::Parser as AdocParser,
    scanner::Scanner,
};

#[cfg(feature = "docx")]
use asciidocr::backends::docx::render_docx;

use cli::{Cli, read_input, read_output};

fn main() {
    let args = Cli::parse();

    // debug is the more detailed level, so it wins when both flags are given
    let verbosity = if args.debug {
        log::LevelFilter::Debug
    } else if args.verbose {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    };

    SimpleLogger::new()
        .with_level(verbosity)
        .with_colors(true)
        .without_timestamps()
        .init()
        .unwrap();

    if let Err(e) = run(args) {
        // (`#` adds the underlying error to the context, if there is any)
        eprintln!("Error converting document: {:#}", e);
        std::process::exit(1)
    }
}

fn run(args: Cli) -> Result<()> {
    if args.file != "-" && !is_asciidoc_file(Path::new(&args.file)) {
        bail!(
            "{} is not an AsciiDoc file (.adoc, .asciidoc or .txt)",
            args.file
        );
    }

    let output = read_output(&args);
    // checked before anything is read, let alone written
    if !args.count
        && let Some(path) = &output
        && args.file != "-"
        && is_same_file(Path::new(&args.file), path)
    {
        bail!(
            "writing to {} would overwrite the input file; name another with -o",
            path.display()
        );
    }

    let mut parser = AdocParser::new(PathBuf::from(&args.file));
    if args.do_not_resolve_targets {
        parser.resolve_targets = false;
    }

    let graph = parser.parse(Scanner::new(&read_input(&args)))?;
    if args.count {
        let count = graph.word_count();
        let words = if count == 1 { "word" } else { "words" };
        println!("{count} {words} in {}", args.file);
        return Ok(());
    }

    match args.backend {
        Backends::Htmlbook => match args.embedded {
            true => render_string(render_htmlbook_embedded(&graph)?, output),
            false => render_string(render_htmlbook(&graph)?, output),
        },

        Backends::Asciidoctor => match args.embedded {
            true => render_string(render_asciidoctor_html_embedded(&graph)?, output),
            false => render_string(render_asciidoctor_html(&graph)?, output),
        },

        #[cfg(feature = "json")]
        Backends::Json => render_string(serde_json::to_string_pretty(&graph)?, output),

        #[cfg(feature = "docx")]
        Backends::Docx => {
            if args.do_not_resolve_targets {
                eprintln!(
                    "Error: the docx backend does not support parsing without target resolution."
                );
                std::process::exit(1)
            }

            if let Some(output_path) = output {
                render_docx(&graph, &output_path)?;
                Ok(())
            } else {
                eprintln!("Error: can't send docx backend to stdout");
                std::process::exit(1)
            }
        }
    }
}

/// Whether a file's extension is one AsciiDoc files have
fn is_asciidoc_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        ["adoc", "asciidoc", "txt"]
            .iter()
            .any(|known| extension.eq_ignore_ascii_case(known))
    })
}

/// Whether two paths name the same file; a path to nothing is no file's
fn is_same_file(first: &Path, second: &Path) -> bool {
    match (fs::canonicalize(first), fs::canonicalize(second)) {
        (Ok(first), Ok(second)) => first == second,
        _ => false,
    }
}

fn render_string(result: String, output_destination: Option<PathBuf>) -> Result<()> {
    match output_destination {
        Some(out_file) => fs::write(&out_file, result)
            .with_context(|| format!("unable to write {}", out_file.display()))?,
        None => println!("{}", result),
    }
    Ok(())
}
