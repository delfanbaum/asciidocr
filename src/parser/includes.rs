//! Include directives: resolving their targets and parsing the files they include

use std::{
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use log::{info, warn};

use crate::scanner::tokens::{Token, TokenType};
use crate::{errors::ParserError, scanner::Scanner};
use crate::{
    graph::asg::Asg,
    utils::{extract_page_ranges, target_and_attrs_from_token},
};

use super::Parser;

impl Parser {
    pub(super) fn parse_include(&mut self, token: Token, asg: &mut Asg) -> Result<(), ParserError> {
        // placeholders
        let mut included_lines: Vec<i32> = vec![];
        let mut include_to_end: bool = false;
        let mut included_tags: Vec<String> = vec![];
        let mut asciidoc_include: bool = false;
        let mut current_tag: Option<String> = None;

        let (target, meta) = target_and_attrs_from_token(&token);
        info!("Parsing included file: {}", &target);
        // check for level offsets in the include
        if let Some(metadata) = meta {
            if let Some(value) = metadata.attributes.get("leveloffset") {
                match self.parse_level_offset(value) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
            if let Some(value) = metadata.attributes.get("lines") {
                included_lines = extract_page_ranges(value);
                if let Some(line) = included_lines.last()
                    && *line == -1
                {
                    include_to_end = true;
                    included_lines.pop();
                }
            }
            if let Some(value) = metadata.attributes.get("tag") {
                included_tags.push(value.clone());
            } // it can either be tag or tags, which is annoying!
            if let Some(value) = metadata.attributes.get("tags") {
                included_tags.extend(value.split(";").map(|tag| tag.to_string()));
            }
        }

        // calculate target, given that it's relative; if there is something on the stack, use
        // that, else use self.origin
        let resolved_target = self.resolve_target(token.line, &target)?;
        self.file_stack.push(target.clone());

        let current_block_stack_len = self.block_stack.len();

        // Are we including another asciidoc file that should be scanned and parsed?
        if matches!(
            target.split('.').next_back().unwrap_or(""),
            "adoc" | "asciidoc" | "txt" // note that per the spec we parse txt as asciidoc
        ) {
            asciidoc_include = true;
        }

        // now parse the file (or skip it, if it can't be read)
        let Some(included) = open_file(resolved_target) else {
            self.file_stack.pop();
            return Ok(());
        };
        for result in Scanner::new_with_stack(&included, self.file_stack.clone()) {
            match result {
                Ok(token) => {
                    // lines
                    if !included_lines.is_empty() {
                        if !included_lines.contains(&(token.line as i32)) {
                            continue;
                        } else if include_to_end
                            && &(token.line as i32) >= included_lines.last().unwrap()
                        {
                            included_lines.clear()
                        }
                    }
                    // tags
                    if !included_tags.is_empty() {
                        if token.token_type() == TokenType::StartTag
                            && let Some(tag) = token.tag()
                            && included_tags.contains(&tag)
                        {
                            current_tag = Some(tag);
                            continue;
                        }
                        if token.token_type() == TokenType::EndTag
                            && let Some(tag) = token.tag()
                            && current_tag == Some(tag)
                        {
                            current_tag = None;
                            continue;
                        }
                        if current_tag.is_none() {
                            continue;
                        }
                    }
                    if asciidoc_include {
                        let token_type = token.token_type();
                        self.token_into(token, asg)?;
                        self.last_token_type = token_type;
                    } else {
                        // allow EOFs to pass through; otherwise just parse as text
                        if matches!(token.token_type(), TokenType::Eof) {
                            self.last_token_type = TokenType::Eof;
                        } else {
                            self.last_token_type = TokenType::Text;
                        }
                        self.parse_text(token)?;
                    }
                }
                Err(e) => return Err(ParserError::Scanner(e)),
            }
        }

        // clean up inlines
        self.add_inlines_to_block_stack()?;

        // get the blocks stack back to where it was; inside a delimited block, though, the blocks
        // from the include wait on the stack, in order, for its closing delimiter to collect them
        while self.block_stack.len() > current_block_stack_len
            && self.open_delimited_block_lines.is_empty()
        {
            self.add_last_block_to_graph(asg)?
        }
        // ...then pop the file off the stack
        self.file_stack.pop();
        Ok(())
    }

    pub(super) fn check_target(&self, token_line: usize, target: &str) -> Result<(), ParserError> {
        if self.resolve_targets {
            match self.resolve_target(token_line, target) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        } else {
            Ok(())
        }
    }

    fn resolve_target(&self, token_line: usize, target: &str) -> Result<PathBuf, ParserError> {
        if !self.file_stack.is_empty() {
            let mut resolved_target = self.origin_directory.clone();
            // may as well follow the rabbit hole
            for file in self.file_stack.iter() {
                if let Some(parent) = PathBuf::from_str(file).unwrap().parent() {
                    resolved_target.push(parent)
                }
            }
            match resolved_target.join(target).canonicalize() {
                Ok(p) => Ok(p),
                Err(_) => Err(ParserError::TargetResolution(token_line, target.into())),
            }
        } else {
            match self.origin_directory.join(target).canonicalize() {
                Ok(p) => Ok(p),
                Err(_) => Err(ParserError::TargetResolution(token_line, target.into())),
            }
        }
    }
}

/// The contents of an included file, or `None` (with a warning) if it can't be read
pub(super) fn open_file<P>(filename: P) -> Option<String>
where
    P: AsRef<Path> + Into<PathBuf> + Debug,
{
    match fs::read_to_string(&filename) {
        Ok(file_string) => Some(file_string),
        Err(e) => {
            warn!("Unable to read file {:?}: {e}", filename);
            None
        }
    }
}
