//! Numbering definitions for lists. A list's items are numbered at the list's nesting depth, so
//! each definition has a level for every depth, indented further at each one.

use crate::graph::lists::NumeralStyle;
use docx_rs::{
    AbstractNumbering, Level, LevelJc, LevelText, NumberFormat, SpecialIndentType, Start,
};

/// How many levels of nesting the definitions cover (Word's maximum)
pub const LEVELS: usize = 9;

/// The left indent of a list item's text at this depth: half an inch per level
pub fn level_indent(level: usize) -> i32 {
    720 * (level as i32 + 1)
}

fn level(level: usize, format: &str, text: String) -> Level {
    Level::new(
        level,
        Start::new(1),
        NumberFormat::new(format),
        LevelText::new(text),
        LevelJc::new("start"),
    )
    .indent(
        Some(level_indent(level)),
        Some(SpecialIndentType::Hanging(360)),
        None,
        None,
    )
}

/// Bullets, which alternate by depth
pub fn bullet_abstract_numbering(id: usize) -> AbstractNumbering {
    let bullets = ["•", "◦", "▪"];
    (0..LEVELS).fold(AbstractNumbering::new(id), |numbering, depth| {
        numbering.add_level(level(depth, "bullet", bullets[depth % 3].to_string()))
    })
}

/// Numbers in the style the list's marker asks for. Like Asciidoctor, the style follows the
/// marker, not the depth (see [`NumeralStyle`]), so every level has the same style.
pub fn ordered_abstract_numbering(id: usize, style: NumeralStyle) -> AbstractNumbering {
    let format = match style {
        NumeralStyle::Arabic => "decimal",
        NumeralStyle::LowerAlpha => "lowerLetter",
        NumeralStyle::LowerRoman => "lowerRoman",
        NumeralStyle::UpperAlpha => "upperLetter",
        NumeralStyle::UpperRoman => "upperRoman",
    };
    (0..LEVELS).fold(AbstractNumbering::new(id), |numbering, depth| {
        numbering.add_level(level(depth, format, format!("%{}.", depth + 1)))
    })
}
