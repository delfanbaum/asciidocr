use std::{env, fs, io, process::exit};
use tera::{Context, Tera};

use asciidocr::{parser::Parser, scanner::Scanner};

fn main() -> Result<(), tera::Error> {
    // take a single arg for simplicity for now; CLI TK
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 {
        println!("{}", render(&args[1])?);
        Ok(())
    } else {
        eprintln!("Usage: asciidocr FILE");
        exit(1)
    }
}

fn render(filename: &str) -> Result<String, tera::Error> {
    // from their docs
    let tera = match Tera::new("templates/**/*.html.tera") {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    let source = open(filename);
    let graph = Parser::new().parse(Scanner::new(&source));
    //let serialized = serde_json::to_string_pretty(&Parser::new().parse(Scanner::new(&source)));
    //let serialized = serde_json::to_string(&Parser::new().parse(Scanner::new(&source)));
    println!("{}", serde_json::to_string_pretty(&graph).expect("Failed to parse graph"));
    Ok(tera.render("htmlbook.html.tera", &Context::from_serialize(&graph)?)?)
}

fn open(filename: &str) -> String {
    match filename {
        "-" => io::read_to_string(io::stdin()).expect("Error reading from stdin"),
        _ => fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to read file: {}", filename)),
    }
}
