use std::{env, fs, process::exit};

use asciidocr::{parser::Parser, scanner::Scanner};

fn main() {
    // take a single arg for simplicity for now; CLI TK
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 {
        run(args[1].clone())
    } else {
        eprintln!("Usage: asciidocr FILE");
        exit(1)
    }
}

fn run(file_path: String) {
    let source =
        fs::read_to_string(&file_path).expect(&format!("Unable to read file: {}", file_path));
    let serialized = serde_json::to_string_pretty(&Parser::new().parse(Scanner::new(&source)));
    println!("{}", serialized.unwrap());
}
