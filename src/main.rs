use std::{env, fs, process::exit};

use asciidocr::scanner::Scanner;

fn main() {
    // take a single arg for simplicity for now; CLI TK
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 {
        run(args[1].clone())
    } else {
        eprintln!("Usage: asciidocr FILE");
        exit(1)
    }

    //scan_and_parse(open(file_path).expect("Unable to open the specified file."));
}

fn run(file_path: String) {
    let source = fs::read_to_string(&file_path).expect(&format!("Unable to read file: {}", file_path));
    let mut s = Scanner::new(&source);
    s.scan_tokens();
    println!("{:?}", s.tokens);
}

// generate errors
// report errors, separate concerns
