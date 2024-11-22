use std::io;

use asciidocr::{parser::Parser, scanner::Scanner};

/// The expectation is that the asciidoc.org TCK calls the adapter, passing stdin and
/// expecting the serialized ASG as stdout
fn main() {
    match io::read_to_string(io::stdin()) {
        Ok(asciidoc) => {
            let graph = Parser::new().parse(Scanner::new(&asciidoc));
            match serde_json::to_string_pretty(&graph) {
                Ok(json) => print!("{}", json),
                Err(e) => {
                    eprintln!("Error producing json: {}", e);
                    std::process::exit(1)
                }
            }
        }
        Err(e) => {
            eprintln!("Error reading from stdin: {}\nThe asciidocr TCK adapter expects to receive content from stdin\nand then returns a json representation of the Abstract Syntax Graph as json to stdout", e);
            std::process::exit(1)
        }
    }
}
