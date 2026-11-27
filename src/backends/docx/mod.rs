//! This module contains document and style-building code for the docx output
mod document;
mod numbering;
mod styles;
mod units;

use std::{fs::File, path::Path};

use docx_rs::Paragraph;

use crate::errors::ConversionError;
use crate::graph::asg::Asg;

pub use self::document::DocxRenderError;

/// docx-rs hardcodes `useFELayout` and `balanceSingleByteDoubleByteWidth` into `settings.xml`.
/// Word and LibreOffice treat those as East Asian layout modes and add extra space at the
/// boundaries of bold/italic runs, which shows up as doubled spaces around emphasis. docx-rs
/// offers no way to turn them off, so we remove them from the generated settings part.
fn strip_east_asian_layout_flags(settings: Vec<u8>) -> Vec<u8> {
    String::from_utf8_lossy(&settings)
        .replace("<w:useFELayout />", "")
        .replace("<w:balanceSingleByteDoubleByteWidth />", "")
        .into_bytes()
}

/// docx-rs always writes its own empty `Normal` style ahead of the styles we add, so
/// `styles.xml` ends up with two definitions. Word uses the first one (and so ignores our font,
/// size, spacing and indent) while LibreOffice uses the last; we drop the empty one.
fn strip_default_normal_style(styles: Vec<u8>) -> Vec<u8> {
    const EMPTY_NORMAL: &str = r#"<w:style w:type="paragraph" w:styleId="Normal"><w:name w:val="Normal" /><w:rPr /><w:pPr><w:rPr /></w:pPr><w:qFormat /></w:style>"#;
    String::from_utf8_lossy(&styles)
        .replacen(EMPTY_NORMAL, "", 1)
        .into_bytes()
}

/// !Experimental! Renders a Docx file. Some [`Asg`] blocks are still unsupported.
pub fn render_docx(graph: &Asg, output_path: &Path) -> Result<(), ConversionError> {
    let file = File::create(output_path)
        .map_err(|e| ConversionError::OutputFile(output_path.to_path_buf(), e))?;
    let mut writer = document::DocxWriter::new();
    let mut docx = document::asciidocr_default_docx();

    // Add document title if present
    if let Some(header) = &graph.header
        && !header.title.is_empty()
    {
        let mut para = Paragraph::new().style("Title");
        para = writer.add_inlines_to_para(para, header.title());
        docx = docx.add_paragraph(para);
    }

    // Add document contents
    for block in graph.blocks.iter() {
        docx = writer.add_block_to_doc(docx, block)?
    }
    let mut xml_docx = docx.build();
    xml_docx.settings = strip_east_asian_layout_flags(xml_docx.settings);
    xml_docx.styles = strip_default_normal_style(xml_docx.styles);
    match xml_docx.pack(file) {
        Ok(_) => Ok(()),
        Err(_) => Err(ConversionError::DocxRender(DocxRenderError::ZipFileError)),
    }
}
