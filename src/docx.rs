use anyhow::Result;
use docx_rs::Docx;
use std::path::Path;

use crate::asg::Asg;

pub fn render_docx(_graph: &Asg, output_path: &Path) -> Result<()> {
    let file = std::fs::File::create(output_path).unwrap();

    let doc = Docx::new();
    // do processing

    doc.build().pack(file)?;
    Ok(())
}
