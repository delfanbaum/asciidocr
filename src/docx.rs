use docx_rust::{document::*, formatting::*, Docx, DocxFile, DocxResult};
use std::{io::Cursor, path::Path};

use crate::{
    asg::Asg,
    blocks::{Block, LeafBlock, LeafBlockName},
    inlines::{Inline, InlineSpanVariant},
    lists::{ListItem, ListVariant},
};

static REFERENCE_DOCX: &[u8] = include_bytes!("../templates/docx/reference.docx");

// TK New idea: made a template DOCX that we read in and _then_ add things to it; seems like there
// are no real defaults on this Docx object.
pub fn render_docx(graph: &Asg, output_path: &Path) -> DocxResult<()> {
    // TK this is going to fail if run outside the crate; need to figure out how to make this
    // relative
    let cursor = Cursor::new(REFERENCE_DOCX);
    let docx = DocxFile::from_reader(cursor)?;
    let donor_doc = docx.parse()?;
    let mut doc = Docx::default();

    doc.styles = donor_doc.styles.clone();

    // TK add a header
    if let Some(header) = &graph.header {
        let mut title = Paragraph::default().property(ParagraphProperty::default().style_id(
            ParagraphStyleId {
                value: std::borrow::Cow::Borrowed("Title"),
            },
        ));
        for inline in header.title() {
            for content in content_from_inline(inline, &mut vec![]) {
                title = title.push(content);
            }
        }
        doc.document.push(title);
    }

    for block in graph.blocks.iter() {
        add_block_to_doc(&mut doc, block);
    }

    doc.write_file(output_path)
        .expect("Error writing to output file");
    Ok(())
}

fn add_block_to_doc<'a>(doc: &mut Docx<'a>, block: &'a Block) {
    match block {
        Block::Section(section) => {
            if !section.title().is_empty() {
                let style_id = format!("Heading{}", section.level);
                let mut title = Paragraph::default().property(
                    ParagraphProperty::default().style_id(ParagraphStyleId {
                        value: style_id.into(),
                    }),
                );
                for inline in section.title() {
                    for content in content_from_inline(inline, &mut vec![]) {
                        title = title.push(content);
                    }
                }
                doc.document.push(title);
            }

            for block in section.blocks.iter() {
                add_block_to_doc(doc, block)
            }
        }
        Block::LeafBlock(block) => {
            doc.document.push(para_from_leafblock(block));
        }
        Block::List(list_block) => {
            for block in &list_block.items {
                if let Block::ListItem(list_item) = block {
                    for para in paras_from_list_item(list_item, list_block.variant.clone()) {
                        doc.document.push(para);
                    }
                };
            }
        }
        _ => {}
    }
    //doc
}

// TK is there a way to manage this so we don't need to recreate the para?
fn para_from_leafblock(block: &LeafBlock) -> Paragraph {
    let mut para = Paragraph::default();
    for inline in block.inlines() {
        for content in content_from_inline(inline, &mut vec![]) {
            para = para.push(content);
        }
    }
    para
}

/// "Simple" lists only for now
fn paras_from_list_item(block: &ListItem, _variant: ListVariant) -> Vec<Paragraph> {
    let mut paras = vec![];
    let mut para =
        Paragraph::default().property(ParagraphProperty::default().style_id(ParagraphStyleId {
            value: "List Paragraph".into(),
        }));
    for inline in block.principal() {
        for content in content_from_inline(inline, &mut vec![]) {
            para = para.push(content);
        }
    }
    // just do all lists as default for now

    paras.push(para);
    for child_block in block.blocks() {
        if let Block::LeafBlock(leaf) = child_block {
            if leaf.name == LeafBlockName::Paragraph {
                let mut child_para = Paragraph::default();
                for inline in block.principal() {
                    for content in content_from_inline(inline, &mut vec![]) {
                        child_para = child_para.push(content);
                    }
                }
                child_para =
                    child_para.property(ParagraphProperty::default().style_id("List Continue"));
                paras.push(child_para);
            } else {
                todo!()
            }
        } else {
            todo!()
        }
    }

    paras
}

fn content_from_inline<'a>(
    inline: Inline,
    variants: &mut Vec<InlineSpanVariant>,
) -> Vec<ParagraphContent<'a>> {
    let mut contents: Vec<ParagraphContent<'a>> = Vec::new();
    match inline {
        Inline::InlineLiteral(lit) => {
            let mut run = Run::default().push_text((lit.value(), TextSpace::Preserve));
            if !variants.is_empty() {
                for variant in variants {
                    match variant {
                        InlineSpanVariant::Strong => {
                            run = run.property(CharacterProperty::default().bold(true));
                        }
                        InlineSpanVariant::Emphasis => {
                            run = run.property(CharacterProperty::default().italics(true));
                        }
                        _ => {} // TODO but not blocking; just adds the literal
                    }
                }
            }
            contents.push(ParagraphContent::Run(run));
        }
        Inline::InlineSpan(span) => {
            variants.push(span.variant);
            for inline in span.inlines {
                contents.extend(content_from_inline(inline, variants))
            }
        }
        Inline::InlineBreak(_) => contents.push(ParagraphContent::Run(
            Run::default().push_break(BreakType::TextWrapping),
        )),
        Inline::InlineRef(iref) => {
            // for now, just append the text; we can handle the actual linking later, as that's
            // more complicated
            for inline in iref.inlines {
                contents.extend(content_from_inline(inline, variants))
            }
        }
    }
    contents
}
