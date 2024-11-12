use std::{collections::HashMap, fmt::Debug};

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

use crate::{nodes::Location, tokens::Token};

static RE_NAMED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(.*?)[=|,](.*)"#).unwrap());

#[derive(PartialEq, Clone, Debug)]
pub enum AttributeType {
    Role,
    Quote,
    Verse,
    Source,
}

impl AttributeType {
    fn from_str(s: &str) -> Option<Self> {
        println!("string to attribue: {}", s);
        match s {
            "role" => Some(AttributeType::Role),
            "quote" => Some(AttributeType::Quote),
            "verse" => Some(AttributeType::Verse),
            "source" => Some(AttributeType::Source),
            _ => None,
        }
    }
}

#[derive(Serialize, PartialEq, Clone, Debug)]
pub struct ElementMetadata {
    pub attributes: HashMap<String, String>,
    pub options: Vec<String>,
    pub roles: Vec<String>,
    /// this is a flag to let us know if it should be applied
    #[serde(skip)]
    pub inline_metadata: bool,
    #[serde(skip)]
    pub declared_type: Option<AttributeType>,
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
        new_block_metadata.process_attributes(attributes);

        new_block_metadata
    }

    pub fn process_attributes(&mut self, mut attributes: Vec<&str>) {
        for (idx, attribute) in attributes.iter_mut().enumerate() {
            match key_values_from_named_attribute(attribute) {
                Ok((key, values)) => {
                    if key == "role".to_string() {
                        for role in values {
                            self.roles.push(role.to_string());
                        }
                    } else {
                        self.attributes.insert(key, values.join(" "));
                    }
                }
                Err(_) => {
                    // i.e., is not a named attribute
                    if attribute.len() > 2 {
                        match &attribute[..2] {
                            "so" | "qu" | "ve" => {
                                self.declared_type = AttributeType::from_str(attribute);
                            }
                            _ => {
                                match self.declared_type {
                                    Some(AttributeType::Source) => {
                                        if idx == 1 {
                                            self.attributes.insert(
                                                String::from("language"),
                                                attribute.trim().into(),
                                            );
                                        }
                                    }
                                    Some(AttributeType::Quote) | Some(AttributeType::Verse) => {
                                        if idx == 1 {
                                            self.attributes.insert(
                                                String::from("attribution"),
                                                String::from(attribute.trim()),
                                            );
                                        } else if idx == 2 {
                                            self.attributes.insert(
                                                String::from("citation"),
                                                String::from(attribute.trim()),
                                            );
                                        } else {
                                            todo!(); // or panic?
                                        }
                                    }
                                    _ => {
                                        if attribute.starts_with('"') {
                                            *attribute = &attribute[1..attribute.len() - 1]
                                        }
                                        self.attributes.insert(
                                            format!("positional_{}", idx + 1),
                                            attribute.to_string(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn key_values_from_named_attribute(attribute: &str) -> Result<(String, Vec<&str>)> {
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
        None => Err(anyhow!("Not a named attribute")),
    }
}

#[cfg(test)]
mod tests {
    use crate::metadata::key_values_from_named_attribute;

    #[test]
    fn values_from_named_attribute_role() {
        assert_eq!(
            ("role".to_string(), vec!["foo", "bar"]),
            key_values_from_named_attribute("role=\"foo bar\"").unwrap()
        )
    }

    #[test]
    fn values_from_named_attribute_any() {
        assert_eq!(
            ("foo".to_string(), vec!["bar"]),
            key_values_from_named_attribute("foo=\"bar\"").unwrap()
        )
    }
}
