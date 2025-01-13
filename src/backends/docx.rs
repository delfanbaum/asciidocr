use docx_rs::*;
use std::{fs::File, path::Path};

use crate::graph::{
    asg::Asg,
    blocks::{Block, BreakVariant, ParentBlockName},
    inlines::{Inline, InlineSpanVariant},
    lists::ListVariant,
};

const DXA_INCH: i32 = 1440; // standard measuring unit in Word

/// !Experimental! Renders a Docx file. Some [`Asg`] blocks are still unsupported.
pub fn render_docx(graph: &Asg, output_path: &Path) -> Result<(), DocxError> {
    let file = File::create(output_path).unwrap();
    let mut writer = DocxWriter::new();

    // always add default style(s) and header; other styles are added as-needed
    let (normal, title) = writer.default_styles();
    let mut docx = Docx::new()
        .add_style(normal)
        .add_style(title)
        .header(writer.get_header())
        .page_size(inches(8.5), inches(11.0))
        .page_margin(writer.get_margins());

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
    current_style: Option<String>,
}

impl DocxWriter {
    fn new() -> Self {
        DocxWriter {
            page_break_before: false,
            current_style: None,
        }
    }

    fn default_styles(&self) -> (Style, Style) {
        let normal = Style {
            style_id: "Normal".into(),
            name: Name::new("Normal"),
            style_type: StyleType::Paragraph,
            run_property: RunProperty::new().size(24),
            paragraph_property: ParagraphProperty::new()
                .line_spacing(LineSpacing::new().line(480))
                .indent(None, Some(SpecialIndentType::FirstLine(720)), None, None),
            table_property: TableProperty::new(),
            table_cell_property: TableCellProperty::new(),
            based_on: None,
            next: None,
            link: None,
        };
        let title = Style::new("Title", StyleType::Paragraph)
            .name("Title")
            .based_on("Normal")
            .indent(None, Some(SpecialIndentType::FirstLine(0)), None, None)
            .bold();
        (normal, title)
    }

    fn add_style(&self, mut docx: Docx, style: Style) -> Docx {
        if docx.styles.find_style_by_id(&style.style_id).is_none() {
            docx = docx.add_style(style)
        }
        docx
    }

    fn add_title_and_text_styles(&self, mut docx: Docx, section_name: &str) -> Docx {
        let title_style = format!("{} Title", section_name);
        let text_style = format!("{} Text", section_name);
        docx = self.add_style(
            docx,
            Style::new(&title_style, StyleType::Paragraph)
                .name(&title_style)
                .based_on("Title")
                .bold(),
        );
        docx = self.add_style(
            docx,
            Style::new(&text_style, StyleType::Paragraph)
                .name(&text_style)
                .based_on("Normal")
                .indent(
                    Some(720),
                    Some(SpecialIndentType::FirstLine(360)),
                    Some(720),
                    None,
                ),
        );
        docx
    }

    fn add_paragraph(&mut self, docx: Docx, mut para: Paragraph) -> Docx {
        if self.page_break_before {
            para = para.page_break_before(true);
            self.page_break_before = false;
        }

        if let Some(style) = &self.current_style {
            if para.property.style.is_none() {
                // don't overwrite styles
                para = para.style(style);
            }
        }

        docx.add_paragraph(para)
    }

    fn add_block_to_doc(&mut self, mut docx: Docx, block: &Block) -> Docx {
        match block {
            Block::Section(section) => {
                if !section.title().is_empty() {
                    let heading_style_name = format!("Heading {}", section.level);
                    let heading_style = Style::new(&heading_style_name, StyleType::Paragraph)
                        .name(&heading_style_name)
                        .based_on("Normal")
                        .bold();
                    docx = self.add_style(docx, heading_style);

                    let mut para = Paragraph::new().style(&heading_style_name);
                    para = add_inlines_to_para(para, section.title());
                    docx = self.add_paragraph(docx, para);
                }

                for block in section.blocks.iter() {
                    docx = self.add_block_to_doc(docx, block)
                }
            }
            Block::List(list) => {
                docx = self.add_style(
                    docx,
                    Style::new("List Paragraph", StyleType::Paragraph)
                        .name("List Paragraph")
                        .based_on("Normal"),
                );
                match list.variant {
                    ListVariant::Ordered | ListVariant::Callout => {
                        docx = docx.add_abstract_numbering(AbstractNumbering::new(1).add_level(
                            Level::new(
                                0,
                                Start::new(1),
                                NumberFormat::new("decimal"),
                                LevelText::new("\t%1."),
                                LevelJc::new("left"),
                            ), // TODO: some indent? Better indent?
                        ));
                        self.current_style = Some("List Numbered".into());
                    }
                    ListVariant::Unordered => {
                        docx = docx.add_abstract_numbering(AbstractNumbering::new(2).add_level(
                            Level::new(
                                0,
                                Start::new(1),
                                NumberFormat::new("decimal"),
                                LevelText::new("%1."),
                                LevelJc::new("left"),
                            ),
                        ));
                        self.current_style = Some("List Bullet".into());
                    }
                }
                for item in list.items.iter() {
                    docx = self.add_block_to_doc(docx, item)
                }
            }
            Block::ListItem(item) => {
                // add principal with the correct variant match
                let mut para = Paragraph::new().style("List Paragraph");
                match &self.current_style {
                    Some(style) => match style.as_str() {
                        "List Numbered" => {
                            para = para.numbering(NumberingId::new(1), IndentLevel::new(0))
                        }
                        _ => {}
                    },
                    _ => {}
                }
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
                    docx = self.add_style(
                        docx,
                        Style::new("Thematic Break", StyleType::Paragraph)
                            .name("Thematic Break")
                            .based_on("Normal")
                            .align(AlignmentType::Center),
                    );
                    docx = self.add_paragraph(
                        docx,
                        Paragraph::new()
                            .style("Thematic Break")
                            .add_run(Run::new().add_text("#")),
                    )
                }
            },
            Block::BlockMacro(_) => todo!(),
            Block::LeafBlock(block) => {
                let mut para = Paragraph::new();
                para = add_inlines_to_para(para, block.inlines());
                docx = self.add_paragraph(docx, para)
            }
            Block::ParentBlock(parent) => match parent.name {
                ParentBlockName::Admonition => {
                    docx = self.add_title_and_text_styles(docx, "Admonition");
                    self.current_style = Some("Admonition Text".into());
                    if let Some(variant) = &parent.variant {
                        docx = self.add_paragraph(
                            docx,
                            Paragraph::new()
                                .style("Admonition Title")
                                .add_run(Run::new().add_text(variant.to_string())),
                        )
                    }
                    for child in parent.blocks.iter() {
                        docx = self.add_block_to_doc(docx, child)
                    }
                    self.current_style = None;
                }
                ParentBlockName::Example => {
                    docx = self.add_title_and_text_styles(docx, "Example");
                    self.current_style = Some("Example Text".into());
                    if !parent.title.is_empty() {
                        let mut title = Paragraph::new().style("Example Title");
                        title = add_inlines_to_para(title, parent.title.clone());
                        docx = self.add_paragraph(docx, title);
                    }
                    for child in parent.blocks.iter() {
                        docx = self.add_block_to_doc(docx, child)
                    }
                    self.current_style = None;
                }
                ParentBlockName::Sidebar => {
                    docx = self.add_title_and_text_styles(docx, "Sidebar");
                    self.current_style = Some("Sidebar Text".into());
                    if !parent.title.is_empty() {
                        let mut title = Paragraph::new().style("Sidebar Title");
                        title = add_inlines_to_para(title, parent.title.clone());
                        docx = self.add_paragraph(docx, title);
                    }
                    for child in parent.blocks.iter() {
                        docx = self.add_block_to_doc(docx, child)
                    }
                    self.current_style = None;
                }
                ParentBlockName::Open => {
                    for child in parent.blocks.iter() {
                        docx = self.add_block_to_doc(docx, child)
                    }
                }
                ParentBlockName::Quote => todo!(),
                ParentBlockName::Table => todo!(),
                // should not appear in this context, so just skip
                ParentBlockName::FootnoteContainer => {}
            },
            Block::BlockMetadata(_) => todo!(),
            Block::TableCell(_) => todo!(),
            _ => todo!(),
        }
        docx
    }

    fn get_header(&self) -> Header {
        Header::new().add_page_num(PageNum::new().align(AlignmentType::Right))
    }

    fn get_margins(&self) -> PageMargin {
        PageMargin::new()
            .top(DXA_INCH)
            .left(DXA_INCH)
            .right(DXA_INCH)
            .bottom(DXA_INCH)
            .header(720)
            .footer(720)
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
            // replace non-significant newlines with space, as it would appear in HTML
            let mut run = Run::new().add_text(lit.value_or_refd_char().replace("\n", " "));
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
            let mut run = Run::new().add_text(lit.value_or_refd_char().replace("\n", " "));
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

fn inches(i: f32) -> u32 {
    (DXA_INCH as f32 * i) as u32
}
