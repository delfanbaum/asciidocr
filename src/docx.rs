use docx_rust::{document::*, formatting::*, styles::Style, Docx, DocxResult};
use std::path::Path;

use crate::{
    asg::Asg,
    blocks::{Block, LeafBlock, LeafBlockName},
    inlines::{Inline, InlineSpanVariant},
    lists::{ListItem, ListVariant},
};

// TK New idea: made a template DOCX that we read in and _then_ add things to it; seems like there
// are no real defaults on this Docx object.
pub fn render_docx(graph: &Asg, output_path: &Path) -> DocxResult<()> {
    let mut doc = Docx::default();

    // TK add a header
    if let Some(header) = &graph.header {
        let mut title = Paragraph::default().property(ParagraphProperty::default().style_id(
            ParagraphStyleId {
                value: std::borrow::Cow::Borrowed("Heading1"),
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
fn paras_from_list_item(block: &ListItem, variant: ListVariant) -> Vec<Paragraph> {
    let mut paras = vec![];
    let mut para = Paragraph::default();
    for inline in block.principal() {
        for content in content_from_inline(inline, &mut vec![]) {
            para = para.push(content);
        }
    }
    match variant {
        ListVariant::Unordered => {
            para = para.property(ParagraphProperty::default().style_id("List Bullet"));
        }
        _ => {
            // just do all other lists as such for now
            para = para.property(ParagraphProperty::default().style_id("List"));
        }
    }
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
    doc.styles.push(
        Style::new(docx_rust::styles::StyleType::Paragraph, "Normal")
            .name("Normal")
            .paragraph(
                ParagraphProperty::default()
                    .indent(Indent {
                        left_chars: None,
                        left: None,
                        right_chars: None,
                        right: None,
                        first_line_chars: None,
                        first_line: Some(720),
                        hanging: None,
                    })
                    .spacing(Spacing::default().line(480 as isize)),
            )
            .character(CharacterProperty::default().fonts(Fonts {
                hint: None,
                ascii: Some("Times New Roman".to_string()),
                east_asia: None,
                h_ansi: Some("Times New Roman".to_string()),
                custom: None,
                ascii_theme: None,
                east_asia_theme: None,
                h_ansi_theme: None,
                custom_theme: None,
            })),
    );
}
