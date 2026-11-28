//! Counting the words in a document, as a word processor counts the text it would show.
//!
//! The text is gathered in runs (a title, a paragraph, a term, a table cell), each counted on its
//! own, so that the words of one don't join those of the next. Within a run, words are separated
//! as LibreOffice separates them: by whitespace and by em and en dashes, and each Chinese,
//! Japanese or Korean character is a word.

use once_cell::sync::Lazy;
use regex::Regex;

use super::{
    blocks::{Block, LeafBlockName},
    inlines::{Inline, InlineLiteralName, InlineRefVariant},
    substitutions::decode_references,
};

/// The number of words in a block, its titles and the blocks inside it
pub(crate) fn count_block(block: &Block) -> usize {
    let own: usize = match block {
        Block::Section(section) => count_run(&section.title()),
        Block::List(list) => count_run(&list.title),
        Block::DList(list) => count_run(&list.title),
        Block::ListItem(item) => count_run(&item.principal),
        Block::DListItem(item) => {
            let terms: usize = item.terms.iter().map(|term| count_run(term)).sum();
            terms + count_run(&item.principal)
        }
        Block::BlockMacro(block_macro) => count_run(&block_macro.caption),
        Block::LeafBlock(leaf) => {
            let text = if leaf.name == LeafBlockName::Pass {
                // passed through as written, so it is markup as much as text
                let source: String = leaf
                    .inlines()
                    .iter()
                    .map(Inline::extract_values_to_string)
                    .collect();
                count_words(&strip_tags(&source))
            } else {
                count_run(&leaf.inlines())
            };
            count_run(&leaf.title) + text
        }
        Block::ParentBlock(parent) => count_run(&parent.title),
        Block::TableCell(cell) => count_run(&cell.inlines),
        Block::Break(_) => 0,
    };
    own + block.child_blocks().iter().map(count_block).sum::<usize>()
}

/// The number of words in a run of inlines
pub(crate) fn count_run(inlines: &[Inline]) -> usize {
    let mut text = String::new();
    push_text(inlines, &mut text);
    count_words(&text)
}

/// Adds the text the inlines show to `text`
fn push_text(inlines: &[Inline], text: &mut String) {
    // passthrough text may come in several pieces, and a tag may be split between them
    let mut raw = String::new();
    for inline in inlines {
        if let Inline::InlineLiteral(literal) = inline
            && matches!(literal.name, InlineLiteralName::Raw)
        {
            raw.push_str(&literal.value());
            continue;
        }
        text.push_str(&strip_tags(&std::mem::take(&mut raw)));
        match inline {
            Inline::InlineLiteral(literal) => text.push_str(&literal.value()),
            Inline::InlineSpan(span) => {
                // a footnote's text is apart from the word it is attached to
                let footnote = span.variant == super::inlines::InlineSpanVariant::Footnote;
                if footnote {
                    text.push(' ');
                }
                push_text(&span.inlines, text);
                if footnote {
                    text.push(' ');
                }
            }
            // an image shows no text (its alt text stands in for it)
            Inline::InlineRef(iref) if iref.variant == InlineRefVariant::Image => text.push(' '),
            // a link without text of its own shows its target
            Inline::InlineRef(iref) if iref.inlines.is_empty() => text.push_str(&iref.target),
            Inline::InlineRef(iref) => push_text(&iref.inlines, text),
            Inline::InlineBreak(_) => text.push(' '),
        }
    }
    text.push_str(&strip_tags(&raw));
}

/// Replaces the HTML tags and comments in passthrough text with spaces
fn strip_tags(source: &str) -> String {
    static TAG: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?s)<!--.*?-->|</?[A-Za-z!][^>]*>").unwrap());
    TAG.replace_all(source, " ").into_owned()
}

/// Whether a character is a word by itself: Chinese characters and punctuation, kana, Korean
/// syllables and the full-width forms
fn is_word_by_itself(character: char) -> bool {
    matches!(
        character,
        '\u{1100}'..='\u{11ff}'
            | '\u{2e80}'..='\u{a4cf}'
            | '\u{ac00}'..='\u{d7af}'
            | '\u{f900}'..='\u{faff}'
            | '\u{ff00}'..='\u{ffef}'
            | '\u{20000}'..='\u{3ffff}'
    )
}

/// The number of words in a piece of text
fn count_words(text: &str) -> usize {
    let mut words = 0;
    let mut in_word = false;
    for character in decode_references(text).chars() {
        if character.is_whitespace() || matches!(character, '\u{2013}' | '\u{2014}') {
            in_word = false;
        } else if is_word_by_itself(character) {
            words += 1;
            in_word = false;
        } else if character != '\u{200b}' && !in_word {
            // (a zero-width space neither separates words nor makes one)
            words += 1;
            in_word = true;
        }
    }
    words
}
