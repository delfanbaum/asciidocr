use std::{env, fs::File, io::Read, process::exit};

use docx_rs::read_docx;

fn main() {
    if let Some(filename) = env::args().last() {
        let mut file = File::open(filename).expect("Can't open file.");
        let mut buf = vec![];
        file.read_to_end(&mut buf).expect("Unable to read file");
        if let Ok(docx) = read_docx(&buf) {
            dbg!(docx.document)
        } else {
            eprintln!("Error reading in docx");
            exit(1)
        };
    } else {
        eprintln!("Please provide a file name");
        exit(1)
    }
}
