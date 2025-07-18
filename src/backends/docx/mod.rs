//! This module contains document and style-building code for the docx output
pub mod document;
pub mod numbering;
pub mod styles;
pub mod units;

use std::{fs::File, path::Path};

use docx_rs::Paragraph;

use crate::graph::asg::Asg;

use self::document::DocxRenderError;

use super::ConversionError;

/// !Experimental! Renders a Docx file. Some [`Asg`] blocks are still unsupported.
pub fn render_docx(graph: &Asg, output_path: &Path) -> Result<(), ConversionError> {
    let file = File::create(output_path).unwrap();
    let mut writer = document::DocxWriter::new();
    let mut docx = document::asciidocr_default_docx();

    // Add document title if present
    if let Some(header) = &graph.header {
        if !header.title.is_empty() {
            let mut para = Paragraph::new().style("Title");
            para = writer.add_inlines_to_para(para, header.title());
            docx = docx.add_paragraph(para);
        }
    }

    // Add document contents
    for block in graph.blocks.iter() {
        docx = writer.add_block_to_doc(docx, block)?
    }
    match docx.build().pack(file) {
        Ok(_) => Ok(()),
        Err(_) => Err(ConversionError::DocxRender(DocxRenderError::ZipFileError)),
    }
}
