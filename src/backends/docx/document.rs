use std::usize;

use docx_rs::{
    AlignmentType, BreakType, Docx, Header, IndentLevel, LineSpacing, Numbering, NumberingId,
    PageMargin, PageNum, Paragraph, Run, RunFonts, RunProperty, Style, Table, TableCell, TableRow,
    VertAlignType,
};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::graph::{
    blocks::{Block, BreakVariant, ParentBlock, ParentBlockName, Section},
    inlines::{Inline, InlineSpanVariant},
    lists::{DListItem, List, ListItem, ListVariant},
};

use super::numbering::add_bullet_abstract_numbering;
use super::styles::DocumentStyles;
use super::units::{inches, DXA_INCH};

static RE_WHITESPACE_NEWLINE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\s*\n"#).unwrap());

pub fn asciidocr_default_docx() -> Docx {
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
pub struct DocxWriter {
    page_break_before: bool,
    abstract_numbering: usize,
    numbering: usize,
    current_style: DocumentStyles,
}

impl DocxWriter {
    pub fn new() -> Self {
        DocxWriter {
            page_break_before: false,
            abstract_numbering: 0,
            numbering: 0,
            current_style: DocumentStyles::Normal,
        }
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

    pub fn add_block_to_doc(&mut self, mut docx: Docx, block: &Block) -> Docx {
        match block {
            Block::Section(section) => docx = self.add_section(docx, section),
            Block::List(list) => docx = self.add_list(docx, list),
            Block::ListItem(item) => docx = self.add_list_item(docx, item),
            Block::DList(list) => {
                for item in list.items.iter() {
                    docx = self.add_block_to_doc(docx, item)
                }
            }
            Block::DListItem(item) => docx = self.add_dlist_item(docx, item),
            Block::Break(block) => match block.variant {
                BreakVariant::Page => {
                    self.page_break_before = true;
                }
                BreakVariant::Thematic => {
                    docx = self.set_style(docx, DocumentStyles::ThematicBreak);
                    docx =
                        self.add_paragraph(docx, Paragraph::new().add_run(Run::new().add_text("#")))
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
                    docx = self.add_parent_block(docx, parent, "Admonition")
                }
                ParentBlockName::Example => docx = self.add_parent_block(docx, parent, "Example"),
                ParentBlockName::Sidebar => docx = self.add_parent_block(docx, parent, "Sidebar"),
                ParentBlockName::Open => {
                    for child in parent.blocks.iter() {
                        docx = self.add_block_to_doc(docx, child)
                    }
                }
                ParentBlockName::Quote => docx = self.add_quote(docx, parent),
                ParentBlockName::Table => docx = self.add_table(docx, parent),
                ParentBlockName::FootnoteContainer => {} // should never appear in this context
            },
            Block::BlockMetadata(_) => todo!(),
            Block::TableCell(_) => {} // handled directly in the parent block
            Block::SectionBody => {}  // not implemented by parser
            Block::NonSectionBlockBody(_) => {} // not implemented by parser
            Block::DiscreteHeading => {} // not implemented by parser
        }
        docx
    }

    fn add_list(&mut self, mut docx: Docx, list: &List) -> Docx {
        self.numbering += 1;
        match list.variant {
            ListVariant::Ordered | ListVariant::Callout => {
                docx = self.set_style(docx, DocumentStyles::OrderedListParagraph(self.numbering))
            }
            ListVariant::Unordered => {
                docx = self.set_style(docx, DocumentStyles::ListParagraph);
                if self.abstract_numbering == 0 {
                    // really only do this once
                    self.abstract_numbering += 1;
                    docx = docx
                        .add_abstract_numbering(add_bullet_abstract_numbering(
                            self.abstract_numbering,
                        ))
                        .add_numbering(Numbering::new(self.numbering, self.abstract_numbering))
                }
            }
        }
        for item in list.items.iter() {
            docx = self.add_block_to_doc(docx, item)
        }
        docx
    }

    fn add_list_item(&mut self, mut docx: Docx, item: &ListItem) -> Docx {
        // add principal with the correct variant match
        let mut para = Paragraph::new();

        para = para
            .style(&self.current_style.style_id())
            .numbering(NumberingId::new(self.numbering), IndentLevel::new(0));
        para = add_inlines_to_para(para, item.principal());
        docx = self.add_paragraph(docx, para);
        // add any children -- TODO style them as list continues
        if !item.blocks.is_empty() {
            for block in item.blocks.iter() {
                docx = self.add_block_to_doc(docx, block)
            }
        }
        docx
    }

    fn add_dlist_item(&mut self, mut docx: Docx, item: &DListItem) -> Docx {
        // add terms
        docx = self.set_style(docx, DocumentStyles::DefinitionTerm);
        let mut terms_para = Paragraph::new();
        terms_para = add_inlines_to_para(terms_para, item.terms());
        docx = self.add_paragraph(docx, terms_para);

        // add principal and anything else
        docx = self.set_style(docx, DocumentStyles::Definition);
        let mut para = Paragraph::new().style(&self.current_style.style_id());
        para = add_inlines_to_para(para, item.principal());
        docx = self.add_paragraph(docx, para);

        // add any children -- TODO style them as list continues
        if !item.blocks.is_empty() {
            for block in item.blocks.iter() {
                docx = self.add_block_to_doc(docx, block)
            }
        }
        docx
    }

    fn add_section(&mut self, mut docx: Docx, section: &Section) -> Docx {
        if !section.title().is_empty() {
            docx = self.set_style(docx, DocumentStyles::Heading(section.level));
            let mut para = Paragraph::new();
            para = add_inlines_to_para(para, section.title());
            docx = self.add_paragraph(docx, para);
        }

        for block in section.blocks.iter() {
            docx = self.add_block_to_doc(docx, block)
        }
        docx
    }

    fn add_parent_block(&mut self, mut docx: Docx, parent: &ParentBlock, name: &str) -> Docx {
        // admonitions
        if let Some(variant) = &parent.variant {
            docx = self.set_style(docx, DocumentStyles::SectionTitle(name.into()));
            docx = self.add_paragraph(
                docx,
                Paragraph::new().add_run(Run::new().add_text(variant.to_string())),
            )
        }
        // examples and sidebars
        if !parent.title.is_empty() {
            docx = self.set_style(docx, DocumentStyles::SectionTitle(name.into()));
            let mut title = Paragraph::new();
            title = add_inlines_to_para(title, parent.title.clone());
            docx = self.add_paragraph(docx, title);
        }
        docx = self.set_style(docx, DocumentStyles::SectionText(name.into()));
        for child in parent.blocks.iter() {
            docx = self.add_block_to_doc(docx, child)
        }
        self.reset_style();
        docx
    }

    fn add_quote(&mut self, mut docx: Docx, parent: &ParentBlock) -> Docx {
        docx = self.set_style(docx, DocumentStyles::Quote);
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
        docx
    }

    fn add_table(&mut self, mut docx: Docx, table: &ParentBlock) -> Docx {
        // TODO: headers on tables
        docx = self.set_style(docx, DocumentStyles::Table);
        let mut cols: usize = 0;
        if let Some(ref metadata) = table.metadata {
            if let Some(col_num) = metadata.attributes.get("cols") {
                cols = col_num.parse().expect("Invalid column designation")
            }
        }
        let mut rows: Vec<TableRow> = vec![];
        let num_cells = table.blocks.len();
        let mut current_row: Vec<TableCell> = vec![];
        for (idx, block) in table.blocks.iter().enumerate() {
            let mut para = Paragraph::new().style(&DocumentStyles::Table.style_id());
            para = add_inlines_to_para(para, block.inlines());
            let cell = TableCell::new().add_paragraph(para);
            current_row.push(cell);
            // see if we need a new row
            if idx > 0 && (idx + 1) % cols == 0 && idx + 1 != num_cells {
                rows.push(TableRow::new(current_row.clone()));
                current_row.clear()
            }
        }
        docx.add_table(Table::new(rows))
    }

    fn add_style(&self, mut docx: Docx, style: Style) -> Docx {
        if docx.styles.find_style_by_id(&style.style_id).is_none() {
            docx = docx.add_style(style)
        }
        docx
    }

    fn set_style(&mut self, docx: Docx, style: DocumentStyles) -> Docx {
        self.current_style = style;
        self.add_style(docx, self.current_style.generate())
    }

    fn reset_style(&mut self) {
        self.current_style = DocumentStyles::Normal;
    }
}

/// Adds inlines to a given paragraph
pub fn add_inlines_to_para(mut para: Paragraph, inlines: Vec<Inline>) -> Paragraph {
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
