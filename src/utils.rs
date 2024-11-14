use std::{fs, io, path::Path};

pub fn is_asciidoc_file(file: &str) -> bool {
    let file_path = Path::new(file);
    match file_path
        .extension()
        .expect(&format!("Invalid file path: {}", file))
        .to_str()
    {
        Some("adoc") | Some("asciidoc") | Some("txt") => true,
        _ => false,
    }
}

pub fn open_file(filename: &str) -> String {
    match filename {
        "-" => io::read_to_string(io::stdin()).expect("Error reading from stdin"),
        _ => fs::read_to_string(filename)
            .unwrap_or_else(|_| panic!("Unable to read file: {}", filename)),
    }
}
