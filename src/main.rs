use std::{env, process::exit};

fn main() {
    // take a single arg for simplicity for now; CLI TK
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        run(args[1].clone())
    } else {
        eprintln!("Usage: asciidocr FILE");
        exit(1)
    }

    //scan_and_parse(open(file_path).expect("Unable to open the specified file."));
}

fn run(_file_path: String) {}

// generate errors
// report errors, separate concerns
