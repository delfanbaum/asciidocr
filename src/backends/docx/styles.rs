use docx_rs::{
    AlignmentType, LineSpacing, Name, ParagraphProperty, RunFonts, RunProperty, SpecialIndentType,
    Style, StyleType, TableCellProperty, TableProperty,
};

#[derive(Debug)]
pub enum DocumentStyles {
    Normal,
    NoSpacing,
    Monospace,
    Title,
    Heading(usize),
    SectionTitle(String),
    SectionText(String),
    Quote,
    Verse,
    ListParagraph,
    OrderedListParagraph(usize),
    DefinitionTerm,
    Definition,
    ThematicBreak,
    Table,
}

impl DocumentStyles {
    pub fn style_id(&self) -> String {
        match self {
            DocumentStyles::Normal => "Normal".into(),
            DocumentStyles::NoSpacing => "No Spacing".into(),
            DocumentStyles::Monospace => "Monospace".into(),
            DocumentStyles::Title => "Title".into(),
            DocumentStyles::Heading(level) => format!("Heading {}", level),
            DocumentStyles::SectionTitle(section_name) => format!("{} Title", section_name),
            DocumentStyles::SectionText(section_name) => format!("{} Text", section_name),
            DocumentStyles::Quote => "Quote".into(),
            DocumentStyles::Verse => "Verse".into(),
            DocumentStyles::ListParagraph => "ListParagraph".into(),
            DocumentStyles::OrderedListParagraph(id) => format!("NumberedListParagraph_{}", id),
            DocumentStyles::DefinitionTerm => "Definition Term".into(),
            DocumentStyles::Definition => "Definition".into(),
            DocumentStyles::ThematicBreak => "ThematicBreak".into(),
            DocumentStyles::Table => "Table".into(),
        }
    }

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
            DocumentStyles::Monospace => Style {
                style_id: "Monospace".into(),
                name: Name::new("Monospace"),
                style_type: StyleType::Paragraph,
                run_property: RunProperty::new()
                    .size(24)
                    .fonts(RunFonts::new().ascii("Courier New")),
                paragraph_property: ParagraphProperty::new().indent(
                    None,
                    Some(SpecialIndentType::FirstLine(0)),
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
                    .indent(
                        Some(720),
                        Some(SpecialIndentType::FirstLine(360)),
                        Some(720),
                        None,
                    )
            }
            DocumentStyles::Quote => Style::new("Quote", StyleType::Paragraph)
                .name("Quote")
                .based_on("No Spacing")
                .italic()
                .next("Normal")
                .indent(
                    Some(720),
                    Some(SpecialIndentType::FirstLine(0)),
                    Some(720),
                    None,
                ),
            DocumentStyles::Verse => Style::new("Verse", StyleType::Paragraph)
                .name("Verse")
                .based_on("No Spacing")
                .italic()
                .next("Normal")
                .indent(
                    Some(0),
                    Some(SpecialIndentType::Hanging(720)),
                    Some(0),
                    None,
                ),
            DocumentStyles::ListParagraph => Style::new("ListParagraph", StyleType::Paragraph)
                .name("ListParagraph")
                .based_on("No Spacing")
                .indent(None, Some(SpecialIndentType::FirstLine(0)), None, None),
            DocumentStyles::OrderedListParagraph(id) => {
                let id = format!("NumberedListParagraph_{}", id);
                Style::new(&id, StyleType::Paragraph)
                    .name(id)
                    .based_on("Normal")
                    .indent(None, Some(SpecialIndentType::FirstLine(0)), None, None)
            }
            DocumentStyles::DefinitionTerm => Style::new("Definition Term", StyleType::Paragraph)
                .name("Definition Term")
                .based_on("Normal")
                .indent(None, Some(SpecialIndentType::FirstLine(0)), None, None)
                .bold(),
            DocumentStyles::Definition => Style::new("Definition", StyleType::Paragraph)
                .name("Definition")
                .based_on("Normal")
                .indent(Some(720), Some(SpecialIndentType::FirstLine(0)), None, None),
            DocumentStyles::ThematicBreak => Style::new("Thematic Break", StyleType::Paragraph)
                .name("Thematic Break")
                .based_on("Normal")
                .align(AlignmentType::Center)
                .bold(),
            DocumentStyles::Table => Style::new("Table", StyleType::Paragraph)
                .name("Table")
                .based_on("Normal")
                .indent(None, Some(SpecialIndentType::FirstLine(0)), None, None),
        }
    }

    // convenience functions for common styles
    pub fn normal() -> Style {
        Self::Normal.generate()
    }
    pub fn no_spacing() -> Style {
        Self::NoSpacing.generate()
    }
    pub fn title() -> Style {
        Self::Title.generate()
    }
}
