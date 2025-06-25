use docx_rs::*;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{fs::File, path::Path};

use crate::graph::{
    asg::Asg,
    blocks::{Block, BreakVariant, ParentBlockName},
    inlines::{Inline, InlineSpanVariant},
    lists::ListVariant,
};

use super::styles::DocumentStyles;

const DXA_INCH: i32 = 1440; // standard measuring unit in Word
static RE_WHITESPACE_NEWLINE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\s*\n"#).unwrap());

/// !Experimental! Renders a Docx file. Some [`Asg`] blocks are still unsupported.
pub fn render_docx(graph: &Asg, output_path: &Path) -> Result<(), DocxError> {
    let file = File::create(output_path).unwrap();
    let mut writer = DocxWriter::new();
    let mut docx = asciidocr_default_docx();

    // Add document title if present
    if let Some(header) = &graph.header {
        if !header.title.is_empty() {
            let mut para = Paragraph::new().style("Title");
            para = add_inlines_to_para(para, header.title());
            docx = docx.add_paragraph(para);
        }
    }

    // Add document contents
    for block in graph.blocks.iter() {
        docx = writer.add_block_to_doc(docx, block)
    }
    docx.build().pack(file)?;
    Ok(())
}

fn asciidocr_default_docx() -> Docx {
    Docx::new()
        .add_style(DocumentStyles::normal())
        .add_style(DocumentStyles::no_spacing())
        .add_style(DocumentStyles::title())
        .header(Header::new().add_page_num(PageNum::new().align(AlignmentType::Right)))
        .page_size(inches(8.5), inches(11.0))
        .page_margin(
            PageMargin::new()
                .top(DXA_INCH)
                .left(DXA_INCH)
                .right(DXA_INCH)
                .bottom(DXA_INCH)
                .header(720)
                .footer(720),
        )
}

// holds some state for us
struct DocxWriter {
    page_break_before: bool,
    abstract_numbering: usize,
    current_style: DocumentStyles,
}

impl DocxWriter {
    fn new() -> Self {
        DocxWriter {
            page_break_before: false,
            abstract_numbering: 0,
            current_style: DocumentStyles::Normal,
        }
    }

    fn add_style(&self, mut docx: Docx, style: Style) -> Docx {
        if docx.styles.find_style_by_id(&style.style_id).is_none() {
            docx = docx.add_style(style)
        }
        docx
    }

    fn add_title_and_text_styles(&self, mut docx: Docx, section_name: &str) -> Docx {
        docx = self.add_style(
            docx,
            DocumentStyles::SectionTitle(section_name.into()).generate(),
        );
        docx = self.add_style(
            docx,
            DocumentStyles::SectionText(section_name.into()).generate(),
        );
        docx
    }

    fn add_paragraph(&mut self, docx: Docx, mut para: Paragraph) -> Docx {
        if self.page_break_before {
            para = para.page_break_before(true);
            self.page_break_before = false;
        }

        // set current style
        para = para.style(&self.current_style.style_id());

        docx.add_paragraph(para)
    }

    fn add_block_to_doc(&mut self, mut docx: Docx, block: &Block) -> Docx {
        match block {
            Block::Section(section) => {
                if !section.title().is_empty() {
                    let heading_style = DocumentStyles::Heading(section.level).generate();
                    let mut para = Paragraph::new().style(&heading_style.style_id);
                    docx = self.add_style(docx, heading_style);

                    para = add_inlines_to_para(para, section.title());
                    docx = self.add_paragraph(docx, para);
                }

                for block in section.blocks.iter() {
                    docx = self.add_block_to_doc(docx, block)
                }
            }
            Block::List(list) => {
                docx = self.add_style(docx, DocumentStyles::ListParagraph.generate());
                match list.variant {
                    ListVariant::Ordered | ListVariant::Callout => {
                        self.abstract_numbering += 1;
                        docx = docx.add_abstract_numbering(
                            AbstractNumbering::new(self.abstract_numbering).add_level(
                                Level::new(
                                    0,
                                    Start::new(1),
                                    NumberFormat::new("decimal"),
                                    LevelText::new("%1."),
                                    LevelJc::new("left"),
                                ), // TODO: some indent? Better indent?
                            ),
                        );
                        self.current_style = DocumentStyles::NumberedListParagraph(1)
                    }
                    ListVariant::Unordered => self.current_style = DocumentStyles::ListParagraph,
                }
                for item in list.items.iter() {
                    docx = self.add_block_to_doc(docx, item)
                }
            }
            Block::ListItem(item) => {
                // add principal with the correct variant match
                let mut para = Paragraph::new();

                if let DocumentStyles::NumberedListParagraph(id) = self.current_style {
                    para = para.style(&self.current_style.style_id())
                        .numbering(NumberingId::new(id), IndentLevel::new(0))
                } else {
                    para = para.style(&self.current_style.style_id());
                }
                para = add_inlines_to_para(para, item.principal());
                dbg!(&para);
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
                            .indent(None, Some(SpecialIndentType::FirstLine(0)), None, None)
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
                if matches!(block.name, crate::graph::blocks::LeafBlockName::Verse) {
                    docx = self.add_style(docx, DocumentStyles::Verse.generate());
                    self.current_style = DocumentStyles::Verse
                }
                let mut para = Paragraph::new();
                para = add_inlines_to_para(para, block.inlines());
                docx = self.add_paragraph(docx, para)
            }
            Block::ParentBlock(parent) => match parent.name {
                ParentBlockName::Admonition => {
                    docx = self.add_title_and_text_styles(docx, "Admonition");
                    self.current_style = DocumentStyles::SectionTitle("Admonition Text".into());
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
                    self.reset_style();
                }
                ParentBlockName::Example => {
                    docx = self.add_title_and_text_styles(docx, "Example");
                    self.current_style = DocumentStyles::SectionTitle("Example Text".into());
                    if !parent.title.is_empty() {
                        let mut title = Paragraph::new().style("Example Title");
                        title = add_inlines_to_para(title, parent.title.clone());
                        docx = self.add_paragraph(docx, title);
                    }
                    for child in parent.blocks.iter() {
                        docx = self.add_block_to_doc(docx, child)
                    }
                    self.reset_style();
                }
                ParentBlockName::Sidebar => {
                    docx = self.add_title_and_text_styles(docx, "Sidebar");
                    self.current_style = DocumentStyles::SectionTitle("Sidebar Text".into());
                    if !parent.title.is_empty() {
                        let mut title = Paragraph::new().style("Sidebar Title");
                        title = add_inlines_to_para(title, parent.title.clone());
                        docx = self.add_paragraph(docx, title);
                    }
                    for child in parent.blocks.iter() {
                        docx = self.add_block_to_doc(docx, child)
                    }
                    self.reset_style();
                }
                ParentBlockName::Open => {
                    for child in parent.blocks.iter() {
                        docx = self.add_block_to_doc(docx, child)
                    }
                }
                ParentBlockName::Quote => {
                    docx = self.add_style(docx, DocumentStyles::Quote.generate());
                    self.current_style = DocumentStyles::Quote;
                    for child in parent.blocks.iter() {
                        docx = self.add_block_to_doc(docx, child)
                    }
                    if let Some(metadata) = &parent.metadata {
                        if let Some(attribution) = metadata.attributes.get("attribution") {
                            let para = Paragraph::new()
                                .line_spacing(LineSpacing::new().after(480))
                                .add_run(Run::new().add_text(format!("— {}\n", attribution)));

                            docx = self.add_paragraph(docx, para)
                        }
                    }
                    self.reset_style();
                }
                ParentBlockName::Table => todo!(),
                // should not appear in this context, so just skip
                ParentBlockName::FootnoteContainer => {}
            },
            Block::BlockMetadata(_) => todo!(),
            Block::TableCell(_) => todo!(),
            Block::SectionBody => todo!(),
            Block::NonSectionBlockBody(_) => todo!(),
            Block::DiscreteHeading => todo!(),
        }
        docx
    }

    fn reset_style(&mut self) {
        self.current_style = DocumentStyles::Normal;
    }
}

/// Adds inlines to a given paragraph
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
            runs.push(
                Run::new()
                    .add_text(RE_WHITESPACE_NEWLINE.replace_all(&lit.value_or_refd_char(), " ")),
            );
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
                        InlineSpanVariant::Code => {
                            run = run.fonts(RunFonts::new().ascii("Courier New"))
                        }
                        InlineSpanVariant::Mark => run = run.highlight("yellow"),
                        InlineSpanVariant::Subscript => {
                            run.run_property =
                                RunProperty::new().vert_align(VertAlignType::SubScript)
                        }
                        InlineSpanVariant::Superscript => {
                            run.run_property =
                                RunProperty::new().vert_align(VertAlignType::SuperScript)
                        }
                        InlineSpanVariant::Footnote => {
                            eprintln!("Footnotes are not well supported; footnote text will be included in-line and higlighted for the time being.");
                            run = run.highlight("blue")
                        }
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
