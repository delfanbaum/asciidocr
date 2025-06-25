use docx_rs::*;

pub enum DocumentStyles {
    Normal,
    NoSpacing,
    Title,
    Heading(usize),
    SectionTitle(String),
    SectionText(String),
    Quote,
    ListParagraph,
    ThematicBreak,
}

impl DocumentStyles {
    pub fn generate(&self) -> Style {
        match self {
            DocumentStyles::Normal => Style {
                style_id: "Normal".into(),
                name: Name::new("Normal"),
                style_type: StyleType::Paragraph,
                run_property: RunProperty::new()
                    .size(24)
                    .fonts(RunFonts::new().ascii("Times New Roman")),
                paragraph_property: ParagraphProperty::new()
                    .line_spacing(LineSpacing::new().line(480))
                    .indent(None, Some(SpecialIndentType::FirstLine(720)), None, None),
                table_property: TableProperty::new(),
                table_cell_property: TableCellProperty::new(),
                based_on: None,
                next: None,
                link: None,
            },
            DocumentStyles::NoSpacing => Style {
                style_id: "No Spacing".into(),
                name: Name::new("No Spacing"),
                style_type: StyleType::Paragraph,
                run_property: RunProperty::new()
                    .size(24)
                    .fonts(RunFonts::new().ascii("Times New Roman")),
                paragraph_property: ParagraphProperty::new().indent(
                    None,
                    Some(SpecialIndentType::FirstLine(720)),
                    None,
                    None,
                ),
                table_property: TableProperty::new(),
                table_cell_property: TableCellProperty::new(),
                based_on: None,
                next: None,
                link: None,
            },
            DocumentStyles::Title => Style::new("Title", StyleType::Paragraph)
                .name("Title")
                .based_on("Normal")
                .indent(None, Some(SpecialIndentType::FirstLine(0)), None, None)
                .bold(),
            DocumentStyles::Heading(level) => {
                let heading_style_name = format!("Heading {}", level);
                Style::new(&heading_style_name, StyleType::Paragraph)
                    .name(&heading_style_name)
                    .based_on("Normal")
                    .indent(None, Some(SpecialIndentType::FirstLine(0)), None, None)
                    .bold()
            }
            DocumentStyles::SectionTitle(section_name) => {
                let style_name = format!("{} Title", section_name);
                Style::new(&style_name, StyleType::Paragraph)
                    .name(&style_name)
                    .based_on("Title")
                    .indent(None, Some(SpecialIndentType::FirstLine(0)), None, None)
                    .bold()
            }
            DocumentStyles::SectionText(section_name) => {
                let style_name = format!("{} Text", section_name);
                Style::new(&style_name, StyleType::Paragraph)
                    .name(&style_name)
                    .based_on("Normal")
                    .bold()
                    .indent(
                        Some(720),
                        Some(SpecialIndentType::FirstLine(360)),
                        Some(720),
                        None,
                    )
            }
            _ => todo!(),
        }
    }
}
