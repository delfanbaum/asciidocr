use clap::Parser;

/// Main entrypoint for asciidocr when called as executable
#[derive(Parser)]
#[command(name = "asciidocr")]
#[command(about="A Rust CLI and library for processing Asciidoc files.", long_about = None)]
pub struct Cli {
    /// Asciidoc file for processing. To provide asciidoc from standard input (stdin), use "-"
    pub file: String,

    /// Flag for returning the asciidoc Abstract Syntax Graph (asg); used for validation
    /// with the official Asciidoc Technology Compatibility Kit (TCK).
    #[arg(short = 'a', long = "tck-adapter")]
    pub adapter: bool,

}
