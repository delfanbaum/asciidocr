use anyhow::Result;
use asciidocr::parse::scan_and_parse;
use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader},
};

fn main() {
    // take a single arg for simplicity for now; CLI TK
    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];
    scan_and_parse(open(file_path).expect("Unable to open the specified file."));
}

fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}
