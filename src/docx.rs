use docx_rust::{document::*, formatting::*, Docx, DocxResult};
use std::path::Path;

use crate::{
    asg::Asg,
    blocks::{Block, LeafBlock},
    inlines::{Inline, InlineSpanVariant},
};

pub fn render_docx(graph: &Asg, output_path: &Path) -> DocxResult<()> {
    let mut doc = Docx::default();

    // TK add a header
    if let Some(header) = &graph.header {
        let mut title = Paragraph::default().property(ParagraphProperty::default().style_id(ParagraphStyleId {value: std::borrow::Cow::Borrowed("Heading1")}));
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

    apply_manuscript_format(&mut doc);

    doc.write_file(output_path)
        .expect("Error writing to output file");
    Ok(())
}

fn add_block_to_doc<'a>(doc: &mut Docx<'a>, block: &'a Block) {
    match block {
        Block::Section(section) => {
            for block in section.blocks.iter() {
                add_block_to_doc(doc, block)
            }
        }
        Block::LeafBlock(block) => {
            doc.document.push(para_from_leafblock(block));
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

// what we want lives in a sectionattr thing
fn apply_manuscript_format(doc: &mut Docx) {
    doc.document.push(SectionProperty {
        rsid_r: None,
        rsid_r_default: None,
        header_footer_references: vec![],
        footnote_property: None,
        endnote_property: None,
        ty: None,
        page_size: Some(PageSize {
            weight: 12240,
            height: 15840,
        }),
        page_margin: Some(PageMargin {
            top: Some(1440),
            right: Some(1440),
            bottom: Some(1440),
            left: Some(1440),
            header: Some(720),
            footer: Some(0),
            gutter: Some(0),
        }),
        paper_source: None,
        page_borders: None,
        line_numbering: None,
        page_numbering: None,
        cols: Some(PageCols { space: Some(720) }),
        form_prot: Some(FormProt { val: Some(false) }),
        v_align: None,
        no_endnote: None,
        title_page: None,
        text_direction: None,
        bidi: None,
        rtl_gutter: None,
        grid: Some(PageGrid {
            ty: None,
            line_pitch: Some(100),
            char_space: None,
        }),
        revision: None,
    });
}
