use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

use crate::{nodes::Location, tokens::Token};

#[derive(Serialize, PartialEq, Clone, Debug)]
pub struct ElementMetadata {
    attributes: HashMap<String, String>,
    options: Vec<String>,
    roles: Vec<String>,
    /// this is a flag to let us know if it should be applied
    #[serde(skip)]
    pub inline_metadata: bool,
    pub location: Vec<Location>,
}

impl ElementMetadata {
    /// Creates BlockMetadata from an attribute list token, which can have the following format:
    /// [positional, named="value inside named", positional]
    pub fn new_inline_meta_from_token(token: Token) -> Self {
        // Regex for parsing named attributes
        let mut new_block_metadata = ElementMetadata {
            attributes: HashMap::new(),
            options: vec![],
            roles: vec![],
            inline_metadata: true,
            location: token.locations().clone(),
        };

        let class_list = token.lexeme[1..token.lexeme.len() - 1].to_string();
        new_block_metadata.roles = class_list
            .split('.')
            .collect::<Vec<&str>>()
            .iter_mut()
            .filter(|s| !s.is_empty() )
            .map(|s| s.to_string())
            .collect();

        new_block_metadata
    }
    pub fn new_block_meta_from_token(token: Token) -> Self {
        // Regex for parsing named attributes
        static RE_NAMED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(.*?)[=|,]"(.*?)\""#).unwrap());

        let mut new_block_metadata = ElementMetadata {
            attributes: HashMap::new(),
            options: vec![],
            roles: vec![],
            inline_metadata: false,
            location: token.locations().clone(),
        };

        let attribute_list = token.lexeme[1..token.lexeme.len() - 1].to_string();
        let attributes: Vec<&str> = attribute_list.split(',').collect();

        for comp in attributes {
            // check if it's a named attributes
            if comp.contains("=\"") {
                let (_, [named, values_str]) = RE_NAMED.captures(comp).unwrap().extract();
                match named {
                    "role" => {
                        let values: Vec<&str> = values_str.split(' ').collect();
                        for role in values {
                            new_block_metadata.roles.push(role.to_string());
                        }
                    }
                    _ => todo!(),
                }
            }
        }
        new_block_metadata
    }
}
