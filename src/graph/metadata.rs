use std::fmt::Display;
use std::{collections::HashMap, fmt::Debug};

use log::info;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

use crate::errors::ParserError;
use crate::graph::nodes::Location;
use crate::scanner::tokens::{Token, TokenType};

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

impl Display for AttributeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeType::Role => write!(f, "Role"),
            AttributeType::Quote => write!(f, "Quote"),
            AttributeType::Verse => write!(f, "Verse"),
            AttributeType::Source => write!(f, "Source"),
            AttributeType::Lines => write!(f, "Lines"),
        }
    }
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
    #[serde(skip)]
    pub line: usize,
}

impl ElementMetadata {
    /// used to check if there's any "there there," as sometimes we just need it for the
    /// declared_type
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty() && self.options.is_empty() && self.roles.is_empty()
    }

    pub fn new_with_role(role_name: String, line: usize) -> Self {
        ElementMetadata {
            attributes: HashMap::new(),
            options: vec![],
            roles: vec![role_name],
            inline_metadata: false,
            declared_type: None,
            location: vec![],
            line,
        }
    }

    pub fn new_with_id(id: String, line: usize) -> Self {
        let mut attrs: HashMap<String, String> = HashMap::with_capacity(1);
        attrs.insert("id".to_string(), id);
        ElementMetadata {
            attributes: attrs,
            options: vec![],
            roles: vec![],
            inline_metadata: false,
            declared_type: None,
            location: vec![],
            line,
        }
    }

    pub fn new_with_id_and_roles(id: String, roles: Vec<String>, line: usize) -> Self {
        let mut attrs: HashMap<String, String> = HashMap::with_capacity(1);
        attrs.insert("id".to_string(), id);
        ElementMetadata {
            attributes: attrs,
            options: vec![],
            roles,
            inline_metadata: false,
            declared_type: None,
            location: vec![],
            line,
        }
    }

    /// Metadata from ID
    pub fn new_inline_with_id(id: String, line: usize) -> Self {
        let mut inline_meta = Self::new_with_id(id, line);
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
            line: token.line,
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
            line: token.line,
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
            match key_values_from_named_attribute(attribute, self.line) {
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
                    if let Some(t) = &self.declared_type {
                        let info = format!(
                            "Additional information found in {} attribute: {}\n      This will be ignored in the output",
                            t,
                            attribute.trim()
                        );
                        info!("{}", info);
                    }
                    self.attributes.insert(
                        format!("additional_quote_info_{}", idx),
                        attribute.trim().to_string(),
                    );
                }
            }
            Some(AttributeType::Lines) => {
                // nothing declares this type yet, and there is nothing to do with its attributes
            }
            _ => {
                // a quoted value loses its quotes (a lone `"` isn't a quoted value)
                if let Some(unquoted) = attribute
                    .strip_prefix('"')
                    .and_then(|rest| rest.strip_suffix('"'))
                {
                    *attribute = unquoted.to_string()
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

/// Splits an attribute list (`role="a b",width=50`) into its entries, keeping quoted values
/// whole
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

/// Splits a named attribute (`role="a b"`) into its name and its space-separated values
pub fn key_values_from_named_attribute(
    attribute: &str,
    line: usize,
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
        None => Err(ParserError::AttributeError(
            line,
            format!("{} does not contain a named attribute", attribute),
        )),
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
    // get rid of the "[]" chars (there are none if the target runs to the end)
    let attributes = target_and_attrs
        .get(target.len() + 1..target_and_attrs.len().saturating_sub(1))
        .unwrap_or_default()
        .to_string();
    let mut metadata: Option<ElementMetadata> = None;
    if !attributes.is_empty() {
        let mut token_metadata = ElementMetadata {
            attributes: HashMap::new(),
            options: vec![],
            roles: vec![],
            inline_metadata: true,
            declared_type: None,
            location: vec![],
            line: token.line,
        };
        token_metadata.process_attributes(extract_attributes(&attributes));
        metadata = Some(token_metadata);
    }
    (target, metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_from_named_attribute_role() {
        assert_eq!(
            ("role".to_string(), vec!["foo", "bar"]),
            key_values_from_named_attribute("role=\"foo bar\"", 1).unwrap()
        )
    }

    #[test]
    fn values_from_named_attribute_any() {
        assert_eq!(
            ("foo".to_string(), vec!["bar"]),
            key_values_from_named_attribute("foo=\"bar\"", 1).unwrap()
        )
    }
}
