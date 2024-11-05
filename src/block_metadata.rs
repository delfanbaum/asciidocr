use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

use crate::{nodes::Location, tokens::Token};

#[derive(Serialize, PartialEq, Clone, Debug)]
pub struct BlockMetadata {
    attributes: HashMap<String, String>,
    options: Vec<String>,
    roles: Vec<String>,
    pub location: Vec<Location>,
}

impl BlockMetadata {
    /// Creates BlockMetadata from an attribute list token, which can have the following format:
    /// [positional, named="value inside named", positional]
    pub fn new_from_token(token: Token) -> Self {
        // Regex for parsing named attributes
        static RE_NAMED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(.*?)[=|,]"(.*?)\""#).unwrap());

        let mut new_block_metadata = BlockMetadata {
            attributes: HashMap::new(),
            options: vec![],
            roles: vec![],
            location: token.locations().clone(),
        };

        let attribute_list = token.lexeme[1..token.lexeme.len() - 1].to_string();
        let attributes: Vec<&str> = attribute_list.split(",").collect();

        for comp in attributes {
            // check if it's a named attributes
            if comp.contains("=\"") {
                let (_, [named, values_str]) = RE_NAMED.captures(comp).unwrap().extract();
                match named {
                    "role" => {
                        let values: Vec<&str> = values_str.split(" ").collect();
                        for role in values {
                            new_block_metadata.roles.push(role.to_string());
                        }
                    }
                    _ => todo!(),
                }
            }
        }
        println!("{:?}", new_block_metadata);

        new_block_metadata
    }
}
