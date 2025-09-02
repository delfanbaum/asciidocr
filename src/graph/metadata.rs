use std::{collections::HashMap, fmt::Debug};

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

use crate::graph::nodes::Location;
use crate::scanner::tokens::{Token, TokenType};
use crate::utils::{extract_attributes, key_values_from_named_attribute};

// just make this quoted, and then pull everything else out
pub static RE_NAMED_QUOTED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(\w*=".*?")"#).unwrap());
pub static RE_NAMED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(.*?)[=|,](.*)"#).unwrap());

#[derive(PartialEq, Clone, Debug)]
pub enum AttributeType {
    Role,
    Quote,
    Verse,
    Source,
    Lines,
}

impl AttributeType {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "role" => Some(AttributeType::Role),
            "quote" => Some(AttributeType::Quote),
            "verse" => Some(AttributeType::Verse),
            "source" => Some(AttributeType::Source),
            _ => None,
        }
    }
}

#[derive(Serialize, PartialEq, Default, Clone, Debug)]
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

    pub fn new_with_role(role_name: String) -> Self {
        ElementMetadata {
            attributes: HashMap::new(),
            options: vec![],
            roles: vec![role_name],
            inline_metadata: false,
            declared_type: None,
            location: vec![],
        }
    }

    pub fn new_with_id(id: String) -> Self {
        let mut attrs: HashMap<String, String> = HashMap::with_capacity(1);
        attrs.insert("id".to_string(), id);
        ElementMetadata {
            attributes: attrs,
            options: vec![],
            roles: vec![],
            inline_metadata: false,
            declared_type: None,
            location: vec![],
        }
    }

    pub fn new_with_id_and_roles(id: String, roles: Vec<String>) -> Self {
        let mut attrs: HashMap<String, String> = HashMap::with_capacity(1);
        attrs.insert("id".to_string(), id);
        ElementMetadata {
            attributes: attrs,
            options: vec![],
            roles,
            inline_metadata: false,
            declared_type: None,
            location: vec![],
        }
    }

    /// Metadata from ID
    pub fn new_inline_with_id(id: String) -> Self {
        let mut inline_meta = Self::new_with_id(id);
        inline_meta.inline_metadata = true;
        inline_meta
    }

    pub fn element_id(&self) -> Option<String> {
        self.attributes.get("id").cloned()
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
        if matches!(token.token_type(), TokenType::BlockAnchor) {
            let id = token.lexeme[2..token.lexeme.len() - 2].to_string(); // skip the "[[" and "]]"
            new_block_metadata.attributes.insert("id".to_string(), id);
        } else {
            let attribute_list = token.lexeme[1..token.lexeme.len() - 1].to_string();
            new_block_metadata.process_attributes(extract_attributes(&attribute_list));
        }
        new_block_metadata
    }

    pub fn add_metadata_from_token(&mut self, token: Token) {
        if matches!(token.token_type(), TokenType::BlockAnchor) {
            let id = token.lexeme[2..token.lexeme.len() - 2].to_string(); // skip the "[[" and "]]"
            if !self.attributes.contains_key("id") {
                self.attributes.insert("id".to_string(), id);
            }
        } else {
            let attribute_list = token.lexeme[1..token.lexeme.len() - 1].to_string();
            self.process_attributes(extract_attributes(&attribute_list));
        }
    }

    pub fn add_metadata_from_other(&mut self, incoming: &ElementMetadata) {
        // combine attributes, deferring to incoming in the case of duplicate keys
        for (key, value) in incoming.attributes.iter() {
            if let Some(extant_key) = self.attributes.get_mut(key) {
                *extant_key = value.to_string()
            } else {
                self.attributes.insert(key.to_string(), value.to_string());
            }
        }
        // combine options and roles
        self.options.extend(incoming.options.clone());
        self.roles.extend(incoming.options.clone());
    }

    pub fn process_attributes(&mut self, mut attributes: Vec<String>) {
        for (idx, attribute) in attributes.iter_mut().enumerate() {
            match key_values_from_named_attribute(attribute) {
                Ok((key, values)) => {
                    if key == *"role" {
                        for role in values {
                            self.roles.push(role.to_string());
                        }
                    } else {
                        self.attributes.insert(key, values.join(" "));
                    }
                }
                Err(_) => {
                    if idx == 0 && attribute.len() >= 2 {
                        match &attribute[..2] {
                            "so" | "qu" | "ve" => {
                                self.declared_type = AttributeType::from_str(attribute);
                                continue;
                            }
                            _ => self.process_attribute(idx, attribute),
                        }
                    } else if !attribute.is_empty() {
                        self.process_attribute(idx, attribute);
                    }
                }
            }
        }
    }

    fn process_attribute(&mut self, idx: usize, attribute: &mut String) {
        match self.declared_type {
            Some(AttributeType::Source) => {
                if idx == 1 {
                    self.attributes
                        .insert(String::from("language"), attribute.trim().into());
                }
            }
            Some(AttributeType::Quote) | Some(AttributeType::Verse) => {
                if idx == 1 {
                    self.attributes
                        .insert(String::from("attribution"), String::from(attribute.trim()));
                } else if idx == 2 {
                    self.attributes
                        .insert(String::from("citation"), String::from(attribute.trim()));
                } else {
                    todo!(); // or panic?
                }
            }
            Some(AttributeType::Lines) => {
                // need to figure out how to
                todo!()
            }
            _ => {
                if attribute.starts_with('"') {
                    *attribute = attribute[1..attribute.len() - 1].to_string()
                }
                self.attributes
                    .insert(format!("positional_{}", idx + 1), attribute.to_string());
            }
        }
    }

    pub fn simplify_cols(&mut self) {
        if let Some(cols_value) = self.attributes.get("cols") {
            if cols_value.contains(',') {
                // cols="1,2,1"
                self.attributes.insert(
                    "cols".to_string(),
                    format!("{}", cols_value.split(',').collect::<Vec<&str>>().len()),
                );
            } else if cols_value.len() == 2 && cols_value[1..] == *"*" {
                self.attributes
                    .insert("cols".to_string(), cols_value[..1].to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
