use core::panic;
use std::collections::HashMap;

use crate::graph::metadata::ElementMetadata;
use crate::scanner::tokens::{Token, TokenType};

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
        let mut image_meta = ElementMetadata {
            attributes: HashMap::new(),
            options: vec![],
            roles: vec![],
            inline_metadata: true,
            declared_type: None,
            location: vec![],
        };
        image_meta.process_attributes(attributes.split(',').collect());
        metadata = Some(image_meta);
    }
    (target, metadata)
}
