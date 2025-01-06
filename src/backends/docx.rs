use docx_rs::*;
use std::{fs::File, path::Path};

use crate::graph::{
    asg::Asg,
    blocks::Block,
    inlines::{Inline, InlineSpanVariant},
};

/// !Experimental! Renders a Docx file. Some [`Asg`] blocks are still unsupported.
pub fn render_docx(graph: &Asg, output_path: &Path) -> Result<(), DocxError> {
    let file = File::create(output_path).unwrap();
    let mut docx = Docx::new();
    for block in graph.blocks.iter() {
        docx = add_block_to_doc(docx, block)
    }
    docx.build().pack(file)?;
    Ok(())
}

fn add_inlines_to_para(mut para: Paragraph, inlines: Vec<Inline>) -> Paragraph {
    for inline in inlines.iter() {
        for run in runs_from_inline(inline) {
            para = para.add_run(run)
        }
    }
    para
}

fn add_block_to_doc(mut docx: Docx, block: &Block) -> Docx {
    match block {
        Block::Section(section) => {
            if !section.title().is_empty() {
                let mut para = Paragraph::new().style(&format!("Heading{}", section.level));
                para = add_inlines_to_para(para, section.title());
                docx = docx.add_paragraph(para)
            }

            for block in section.blocks.iter() {
                docx = add_block_to_doc(docx, block)
            }
        }
        Block::LeafBlock(block) => {
            let mut para = Paragraph::new();
            para = add_inlines_to_para(para, block.inlines());
            docx = docx.add_paragraph(para)
        }
        _ => todo!(),
    }
    docx
}

fn runs_from_inline(inline: &Inline) -> Vec<Run> {
    let mut variants: Vec<&InlineSpanVariant> = vec![];
    let mut runs: Vec<Run> = vec![];
    match inline {
        Inline::InlineLiteral(lit) => {
            let mut run = Run::new().add_text(lit.value_or_refd_char());
            if !variants.is_empty() {
                for variant in variants {
                    match variant {
                        InlineSpanVariant::Strong => run = run.bold(),
                        InlineSpanVariant::Emphasis => run = run.italic(),
                        _ => {} // TODO but not blocking; just adds the literal
                    }
                }
            }
            runs.push(run)
        }
        Inline::InlineSpan(span) => {
            variants.push(&span.variant);
            for inline in span.inlines.iter() {
                runs.extend(runs_from_inline_with_variant(inline, &mut variants))
            }
        }
        Inline::InlineBreak(_) => runs.push(Run::new().add_break(BreakType::TextWrapping)),
        Inline::InlineRef(iref) => {
            // for now, just append the text; we can handle the actual linking later, as that's
            // more complicated
            for inline in iref.inlines.iter() {
                runs.extend(runs_from_inline_with_variant(inline, &mut variants))
            }
        }
    }
    runs
}

fn runs_from_inline_with_variant<'a>(
    inline: &'a Inline,
    variants: &mut Vec<&'a InlineSpanVariant>,
) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    match inline {
        Inline::InlineLiteral(lit) => {
            let mut run = Run::new().add_text(lit.value_or_refd_char());
            if !variants.is_empty() {
                for variant in variants {
                    match variant {
                        InlineSpanVariant::Strong => run = run.bold(),
                        InlineSpanVariant::Emphasis => run = run.italic(),
                        _ => {} // TODO but not blocking; just adds the literal
                    }
                }
            }
            runs.push(run)
        }
        Inline::InlineSpan(span) => {
            variants.push(&span.variant);
            for inline in span.inlines.iter() {
                runs.extend(runs_from_inline_with_variant(inline, variants))
            }
        }
        Inline::InlineBreak(_) => runs.push(Run::new().add_break(BreakType::TextWrapping)),
        Inline::InlineRef(iref) => {
            // for now, just append the text; we can handle the actual linking later, as that's
            // more complicated
            for inline in iref.inlines.iter() {
                runs.extend(runs_from_inline_with_variant(inline, variants))
            }
        }
    }
    runs
}
