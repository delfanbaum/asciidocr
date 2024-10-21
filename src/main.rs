use std::{
    env,
    fs::{self},
    io,
    process::exit,
};

use asciidocr::{parser::Parser, scanner::Scanner};

fn main() {
    // take a single arg for simplicity for now; CLI TK
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 {
        run(&args[1])
    } else {
        eprintln!("Usage: asciidocr FILE");
        exit(1)
    }
}

fn run(filename: &str) {
    let source = open(filename);
    let serialized = serde_json::to_string_pretty(&Parser::new().parse(Scanner::new(&source)));
    //let serialized = serde_json::to_string(&Parser::new().parse(Scanner::new(&source)));
    println!("{}", serialized.unwrap());
}

fn open(filename: &str) -> String {
    match filename {
        "-" => io::read_to_string(io::stdin()).expect("Error reading from stdin"),
        _ => fs::read_to_string(filename).expect(&format!("Unable to read file: {}", filename)),
    }
}
