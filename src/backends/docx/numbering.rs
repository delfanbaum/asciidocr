use docx_rs::*;
// Temporary function to get this working
//
pub fn add_bullet_abstract_numbering(id: usize) -> AbstractNumbering {
    AbstractNumbering::new(id).add_level(Level::new(
        0,
        Start::new(1),
        NumberFormat::new("bullet"),
        LevelText::new("•"),
        LevelJc::new("start"), // I have no clue if this is right
    ))
}

// // Add the concrete numbering instance that references the abstract numbering.
// // The id (1 in this case) links this specific numbering to the abstract definition.
// docx = docx.add_numbering(docx_rs::Numbering::new(1, 1));
