use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

use crate::{nodes::Location, tokens::Token};

static RE_NAMED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(.*?)[=|,]"(.*?)\""#).unwrap());

#[derive(Serialize, PartialEq, Clone, Debug)]
pub struct ElementMetadata {
    attributes: HashMap<String, String>,
    options: Vec<String>,
    roles: Vec<String>,
    /// this is a flag to let us know if it should be applied
    #[serde(skip)]
    pub inline_metadata: bool,
    #[serde(skip)]
    pub declared_type: Option<String>,
    pub location: Vec<Location>,
}

impl ElementMetadata {
    /// used to check if there's any "there there," as sometimes we just need it for the
    /// declared_type
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty() && self.options.is_empty() && self.roles.is_empty()
    }

    /// [positional, named="value inside named", positional]
    pub fn new_inline_meta_from_token(token: Token) -> Self {
        // Regex for parsing named attributes
        let mut new_block_metadata = ElementMetadata {
            attributes: HashMap::new(),
            options: vec![],
            roles: vec![],
            inline_metadata: true,
            declared_type: None,
            location: token.locations().clone(),
        };

        let class_list = token.lexeme[1..token.lexeme.len() - 1].to_string();
        new_block_metadata.roles = class_list
            .split('.')
            .collect::<Vec<&str>>()
            .iter_mut()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        new_block_metadata
    }
    pub fn new_block_meta_from_token(token: Token) -> Self {
        // Regex for parsing named attributes

        let mut new_block_metadata = ElementMetadata {
            attributes: HashMap::new(),
            options: vec![],
            roles: vec![],
            inline_metadata: false,
            declared_type: None,
            location: token.locations().clone(),
        };

        let attribute_list = token.lexeme[1..token.lexeme.len() - 1].to_string();
        let attributes: Vec<&str> = attribute_list.split(',').collect();

        // determine kind of thing
        if !attributes.is_empty() {
            match &attributes[0][..4] {
                "role" => {
                    new_block_metadata.declared_type = Some(String::from("role"));
                    for role in values_from_named_attribute(attributes[0]) {
                        new_block_metadata.roles.push(role.to_string());
                    }
                }
                "sour" => {
                    new_block_metadata.declared_type = Some(String::from("source"));
                    // if a sour source block, see if there's a language
                    if attributes.len() >= 2 {
                        new_block_metadata
                            .attributes
                            .insert(String::from("language"), attributes[1].trim().into());
                    }
                }
                "quot" | "vers" => {
                    new_block_metadata.declared_type = Some(String::from(attributes[0]));
                    if attributes.len() >= 2 {
                        for (idx, attr) in attributes[1..].iter().enumerate() {
                            match idx {
                                0 => {
                                    new_block_metadata
                                        .attributes
                                        .insert(String::from("attribution"), attr.trim().into());
                                }
                                1 => {
                                    new_block_metadata
                                        .attributes
                                        .insert(String::from("citation"), attr.trim().into());
                                }
                                _ => todo!(), // or panic?
                            }
                        }
                    }
                }
                _ => {
                    todo!()
                }
            }
        }

        new_block_metadata
    }
}

fn values_from_named_attribute(attribute: &str) -> Vec<&str> {
    let (_, [named, values_str]) = RE_NAMED.captures(attribute).unwrap().extract();
    match named {
        "role" => values_str.split(' ').collect::<Vec<&str>>(),
        _ => todo!(),
    }
}
