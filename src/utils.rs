use std::{fs, io, path::Path};

use log::warn;

pub fn is_asciidoc_file(file: &str) -> bool {
    let file_path = Path::new(file);
    matches!(
        file_path
            .extension()
            .unwrap_or_else(|| panic!("Invalid file path: {}", file))
            .to_str(),
        Some("adoc") | Some("asciidoc") | Some("txt")
    )
}

pub fn open_file(filename: &str) -> String {
    match filename {
        "-" => io::read_to_string(io::stdin()).expect("Error reading from stdin"),
        _ => match fs::read_to_string(filename) {
            Ok(file_string) => file_string,
            Err(e) => {
                warn!("Unable to read file {filename}: {e}");
                std::process::exit(1)
            }
        },
    }
}
