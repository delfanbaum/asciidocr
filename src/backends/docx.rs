use docx_rs::*;
use std::{fs::File, path::Path};

use crate::graph::{
    asg::Asg,
    blocks::{Block, BreakVariant},
    inlines::{Inline, InlineSpanVariant},
    lists::ListVariant,
};

/// !Experimental! Renders a Docx file. Some [`Asg`] blocks are still unsupported.
pub fn render_docx(graph: &Asg, output_path: &Path) -> Result<(), DocxError> {
    let file = File::create(output_path).unwrap();
    let mut writer = DocxWriter::new();
    let mut docx = Docx::new();
    let (normal, heading1, heading2, heading3, heading4, heading5, title) = writer.get_styles();
    docx = docx
        .header(writer.get_header())
        .add_style(normal)
        .add_style(heading1)
        .add_style(heading2)
        .add_style(heading3)
        .add_style(heading4)
        .add_style(heading5)
        .add_style(title);

    if let Some(header) = &graph.header {
        if !header.title.is_empty() {
            let mut para = Paragraph::new().style("Title");
            para = add_inlines_to_para(para, header.title());
            docx = docx.add_paragraph(para);
        }
    }
    for block in graph.blocks.iter() {
        docx = writer.add_block_to_doc(docx, block)
    }
    docx.build().pack(file)?;
    Ok(())
}

// holds some state for us
struct DocxWriter {
    page_break_before: bool,
    current_list_style: String, // hold it as a stack for nested lists
}

impl DocxWriter {
    fn new() -> Self {
        DocxWriter {
            page_break_before: false,
            current_list_style: "".into(),
        }
    }

    fn get_styles(&self) -> (Style, Style, Style, Style, Style, Style, Style) {
        let normal = Style {
            style_id: "Normal".into(),
            name: Name::new("Normal"),
            style_type: StyleType::Paragraph,
            run_property: RunProperty::new().size(24),
            paragraph_property: ParagraphProperty::new().line_spacing(LineSpacing::new().line(480)),
            table_property: TableProperty::new(),
            table_cell_property: TableCellProperty::new(),
            based_on: None,
            next: None,
            link: None,
        };
        let heading1 = Style::new("Heading 1", StyleType::Paragraph)
            .name("Heading 1")
            .based_on("Normal")
            .bold();
        let heading2 = Style::new("Heading 2", StyleType::Paragraph)
            .name("Heading 2")
            .based_on("Normal")
            .bold();
        let heading3 = Style::new("Heading 3", StyleType::Paragraph)
            .name("Heading 3")
            .based_on("Normal")
            .bold();
        let heading4 = Style::new("Heading 4", StyleType::Paragraph)
            .name("Heading 4")
            .based_on("Normal")
            .bold();
        let heading5 = Style::new("Heading 5", StyleType::Paragraph)
            .name("Heading 5")
            .based_on("Normal")
            .bold();
        let title = Style::new("Title", StyleType::Paragraph)
            .name("Title")
            .based_on("Normal")
            .bold();
        (
            normal, heading1, heading2, heading3, heading4, heading5, title,
        )
    }

    fn add_paragraph(&mut self, docx: Docx, mut para: Paragraph) -> Docx {
        if self.page_break_before {
            para = para.page_break_before(true);
            self.page_break_before = false;
        }
        docx.add_paragraph(para)
    }

    fn add_block_to_doc(&mut self, mut docx: Docx, block: &Block) -> Docx {
        match block {
            Block::Section(section) => {
                if !section.title().is_empty() {
                    let mut para = Paragraph::new().style(&format!("Heading {}", section.level));
                    para = add_inlines_to_para(para, section.title());
                    docx = self.add_paragraph(docx, para)
                }

                for block in section.blocks.iter() {
                    docx = self.add_block_to_doc(docx, block)
                }
            }
            Block::List(list) => {
                match list.variant {
                    ListVariant::Ordered | ListVariant::Callout => {
                        self.current_list_style = "List Paragraph".into();
                    }
                    ListVariant::Unordered => self.current_list_style = "List Paragraph".into(),
                }
                for item in list.items.iter() {
                    docx = self.add_block_to_doc(docx, item)
                }
            }
            Block::ListItem(item) => {
                // add principal with the correct variant match
                let mut para = Paragraph::new().style(&self.current_list_style);
                para = add_inlines_to_para(para, item.principal());
                docx = self.add_paragraph(docx, para);
                // add any children -- TODO style them as list continues
                if !item.blocks.is_empty() {
                    for block in item.blocks.iter() {
                        docx = self.add_block_to_doc(docx, block)
                    }
                }
            }
            Block::DList(_) => todo!(),
            Block::DListItem(_) => todo!(),
            Block::Break(block) => match block.variant {
                BreakVariant::Page => {
                    self.page_break_before = true;
                }
                BreakVariant::Thematic => {
                    docx = self.add_paragraph(
                        docx,
                        Paragraph::new()
                            .add_run(Run::new().add_text("#"))
                            .align(AlignmentType::Center),
                    )
                }
            },
            Block::BlockMacro(_) => todo!(),
            Block::LeafBlock(block) => {
                let mut para = Paragraph::new();
                para = add_inlines_to_para(para, block.inlines());
                docx = self.add_paragraph(docx, para)
            }
            Block::ParentBlock(_) => todo!(),
            Block::BlockMetadata(_) => todo!(),
            Block::TableCell(_) => todo!(),
            _ => todo!(),
        }
        docx
    }

    fn get_header(&self) -> Header {
        Header::new().add_page_num(
            PageNum::new()
                .wrap("none")
                .v_anchor("text")
                .h_anchor("margin")
                .x_align("right"),
        )
    }
}

fn add_inlines_to_para(mut para: Paragraph, inlines: Vec<Inline>) -> Paragraph {
    for inline in inlines.iter() {
        for run in runs_from_inline(inline) {
            para = para.add_run(run)
        }
    }
    para
}

/// Creates runs from inlines, called from a block
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

/// The version that allows for recursion, specifically nested inline spans
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
