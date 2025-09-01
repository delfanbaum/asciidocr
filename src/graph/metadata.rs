use std::ops::Range;
use std::{collections::HashMap, fmt::Debug};

use anyhow::{Result, anyhow};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

use crate::graph::nodes::Location;
use crate::scanner::tokens::{Token, TokenType};

// just make this quoted, and then pull everything else out
static RE_NAMED_QUOTED: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(\w*=".*?")"#).unwrap());
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
            let mut non_quoted_key_values = attribute_list.clone();
            let mut attributes: Vec<&str> = vec![];
            // TK "1,2,4" should be a single attribute, not "1,", 2, 4"
            for quoted_attr in RE_NAMED_QUOTED.captures_iter(&attribute_list) {
                let (total, [_]) = quoted_attr.extract();
                attributes.push(total);
                non_quoted_key_values = non_quoted_key_values.replace(total, "");
            }
            attributes.extend(non_quoted_key_values.split(',').collect::<Vec<&str>>());

            new_block_metadata.process_attributes(attributes);
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
            let attributes: Vec<&str> = attribute_list.split(',').collect();
            self.process_attributes(attributes);
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

    pub fn process_attributes(&mut self, mut attributes: Vec<&str>) {
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

    fn process_attribute(&mut self, idx: usize, attribute: &mut &str) {
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
            _ => {
                if attribute.starts_with('"') {
                    *attribute = &attribute[1..attribute.len() - 1]
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

pub fn extract_page_ranges(ranges_str: &str) -> Option<Vec<usize>> {
    let mut ranges: Vec<usize> = vec![];

    for range in ranges_str.split(";") {
        let parts: Vec<&str> = range.split("..").collect();
        if !parts.is_empty() {
            let start = parts[0].parse::<usize>().ok()?;
            if parts.len() == 2 {
                let mut end = parts[1].parse::<usize>().ok()?;
                end += 1; // because we want an inclusive range 
                ranges.extend(start..end)
            } else {
                ranges.push(start)
            }
        }
    }

    if !ranges.is_empty() {
        Some(ranges)
    } else {
        None
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
