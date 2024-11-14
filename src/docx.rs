use docx_rust::{
    document::{Paragraph, Run},
    Docx, DocxResult,
};
use std::path::Path;

use crate::{
    asg::Asg,
    blocks::{Block, LeafBlock},
    inlines::Inline,
};

pub fn render_docx(graph: &Asg, output_path: &Path) -> DocxResult<()> {
    let mut doc = Docx::default();

    for block in graph.blocks.iter() {
        match block {
            Block::LeafBlock(block) => {
                doc.document.push(para_from_leafblock(block));
            }
            _ => {}
        }
    }

    doc.write_file(output_path).expect("Error writing to output file");
    Ok(())
}

fn para_from_leafblock<'a>(block: &LeafBlock) -> Paragraph {
    let mut para = Paragraph::default();
    for inline in block.inlines() {
        para = para.push(run_from_inline(inline));
    }
    para
}

fn run_from_inline<'a>(inline: Inline) -> Run<'a> {
    match inline {
        Inline::InlineLiteral(lit) => Run::default().push_text(lit.value()),
        _ => todo!(),
    }
}
