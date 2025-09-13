use core::panic;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

use crate::errors::ParserError;
use crate::graph::metadata::{ElementMetadata, RE_NAMED, RE_NAMED_QUOTED};
use crate::scanner::tokens::{Token, TokenType};

pub static RE_LINE_RANGES: Lazy<Regex> = Lazy::new(|| Regex::new(r#"([^,;]*)"#).unwrap());

pub fn key_values_from_named_attribute(
    attribute: &str,
) -> Result<(String, Vec<&str>), ParserError> {
    match RE_NAMED.captures(attribute) {
        Some(captures) => {
            let (_, [named, mut values_str]) = captures.extract();
            // remove quotes values
            if values_str.starts_with('"') {
                values_str = &values_str[1..values_str.len() - 1]
            }
            Ok((
                named.to_string(),
                values_str.split(' ').collect::<Vec<&str>>(),
            ))
        }
        None => Err(ParserError::AttributeError(format!(
            "{} does not contain a named attribute",
            attribute
        ))),
    }
}

/// Given a foo:target.foo[attributes] token, return the target and metadata
pub fn target_and_attrs_from_token(token: &Token) -> (String, Option<ElementMetadata>) {
    let target_and_attrs = match token.token_type() {
        TokenType::BlockImageMacro => {
            token.text()[7..].to_string() // after image::
        }
        TokenType::InlineImageMacro => {
            token.text()[6..].to_string() // after image:
        }
        TokenType::Include => {
            token.text()[9..].to_string() // after include::
        }
        _ => panic!("Invalid token provided to target_and_attrs_from_token"),
    };

    let target: String = target_and_attrs.chars().take_while(|c| c != &'[').collect();
    // get rid of the "[]" chars
    let attributes = target_and_attrs[target.len() + 1..target_and_attrs.len() - 1].to_string();
    let mut metadata: Option<ElementMetadata> = None;
    if !attributes.is_empty() {
        let mut token_metadata = ElementMetadata {
            attributes: HashMap::new(),
            options: vec![],
            roles: vec![],
            inline_metadata: true,
            declared_type: None,
            location: vec![],
        };
        token_metadata.process_attributes(extract_attributes(&attributes));
        metadata = Some(token_metadata);
    }
    (target, metadata)
}

/// Extracts included page ranges from the "lines=" attribute of an include directive
pub fn extract_page_ranges(ranges_str: &str) -> Vec<i32> {
    let mut ranges: Vec<i32> = vec![];

    let mut captured_ranges = RE_LINE_RANGES.captures_iter(ranges_str).peekable();
    while let Some(range) = captured_ranges.next() {
        let (range_str, [_]) = range.extract();
        let parts: Vec<&str> = range_str.split("..").collect();
        if !parts.is_empty() {
            let start = parts[0].parse::<i32>().unwrap_or(0);
            if parts.len() == 2 {
                // if there is an end value
                if let Ok(mut end) = parts[1].parse::<i32>() {
                    if end == -1 {
                        // if we're reading to the end, read to the end
                        ranges.push(start);
                        ranges.push(-1);
                    } else {
                        end += 1; // because we want an inclusive range 
                        ranges.extend(start..end)
                    }
                } else if captured_ranges.peek().is_none() {
                    ranges.push(start);
                    ranges.push(-1); // signals that we read to the end
                }
            } else {
                ranges.push(start);
            }
        }
    }
    ranges
}

pub fn extract_attributes(attribute_list: &str) -> Vec<String> {
    let mut attributes: Vec<String> = vec![];
    let mut non_quoted_key_values = attribute_list.to_owned();
    // TK "1,2,4" should be a single attribute, not "1,", 2, 4"
    for quoted_attr in RE_NAMED_QUOTED.captures_iter(attribute_list) {
        let (total, [_]) = quoted_attr.extract();
        attributes.push(total.to_owned());
        non_quoted_key_values = non_quoted_key_values.replace(total, "");
    }
    attributes.extend(non_quoted_key_values.split(',').map(|s| s.to_string()));
    attributes
}
