use docx_rs::{AlignmentType, LineSpacing, RunFonts, SpecialIndentType, Style, StyleType};

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
    ListParagraphContinue,
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
            DocumentStyles::ListParagraphContinue => "ListParagraphContinue".into(),
            DocumentStyles::OrderedListParagraph(id) => format!("NumberedListParagraph_{}", id),
            DocumentStyles::DefinitionTerm => "Definition Term".into(),
            DocumentStyles::Definition => "Definition".into(),
            DocumentStyles::ThematicBreak => "ThematicBreak".into(),
            DocumentStyles::Table => "Table".into(),
        }
    }

    pub fn generate(&self) -> Style {
        match self {
            // built with the builder rather than struct literals, because docx-rs adds fields to
            // `Style` in patch releases (0.4.22 did), which breaks struct literals
            DocumentStyles::Normal => Style::new("Normal", StyleType::Paragraph)
                .name("Normal")
                .size(24)
                .fonts(RunFonts::new().ascii("Times New Roman"))
                .line_spacing(LineSpacing::new().line(480))
                .indent(None, Some(SpecialIndentType::FirstLine(720)), None, None),
            DocumentStyles::NoSpacing => Style::new("No Spacing", StyleType::Paragraph)
                .name("No Spacing")
                .size(24)
                .fonts(RunFonts::new().ascii("Times New Roman"))
                .indent(None, Some(SpecialIndentType::FirstLine(720)), None, None),
            DocumentStyles::Monospace => Style::new("Monospace", StyleType::Paragraph)
                .name("Monospace")
                .size(24)
                .fonts(RunFonts::new().ascii("Courier New"))
                .indent(None, Some(SpecialIndentType::FirstLine(0)), None, None),
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
            DocumentStyles::ListParagraphContinue => {
                Style::new("ListParagraphContinue", StyleType::Paragraph)
                    .name("ListParagraphContinue")
                    .based_on("No Spacing")
                    .indent(Some(720), Some(SpecialIndentType::FirstLine(0)), None, None)
            }
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
