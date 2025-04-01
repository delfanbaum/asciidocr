// For each kind of block, figure out the display and add it to a "Line", which will then be added
// to a Paragraph object that serves as the anchor for this kind of display.

use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};

use crate::graph::blocks::Block;
use crate::graph::inlines::{Inline, InlineSpanVariant};

pub fn line_from_block(block: &Block) -> Line {
    let line = Line::from(spans_from_inlines(block.inlines()));
    line
    //match block {
    //    _ => todo!(),
    //}
}

// TODO spans within spans, etc.
fn spans_from_inlines_colorized<'a>(inlines: Vec<Inline>, color: Color) -> Vec<Span<'a>> {
    let mut spans = vec![];
    for inline in inlines {
        match inline {
            Inline::InlineLiteral(_) => spans.push(Span::styled(
                inline.extract_values_to_string(),
                Style::default().fg(color),
            )),
            Inline::InlineSpan(_) => spans.push(Span::styled(
                inline.extract_values_to_string(),
                Style::default().fg(color),
            )),
            Inline::InlineBreak(_) => spans.push(Span::raw("\n")),
            _ => todo!(),
        }
    }
    return spans;
}

fn spans_from_inlines<'a>(inlines: Vec<&Inline>) -> Vec<Span<'a>> {
    let mut spans = vec![];
    for inline in inlines {
        match inline {
            Inline::InlineLiteral(literal) => spans.push(Span::raw(literal.value())),
            Inline::InlineSpan(ref span) => match span.variant {
                InlineSpanVariant::Strong => spans.push(Span::styled(
                    inline.extract_values_to_string(),
                    Style::new().bold(),
                )),
                InlineSpanVariant::Emphasis => spans.push(Span::styled(
                    inline.extract_values_to_string(),
                    Style::new().italic(),
                )),
                InlineSpanVariant::Code => spans.push(Span::styled(
                    inline.extract_values_to_string(),
                    Style::new().red(),
                )),
                InlineSpanVariant::Mark => spans.push(Span::styled(
                    inline.extract_values_to_string(),
                    Style::new().yellow(),
                )),
                _ => todo!(),
            },
            Inline::InlineBreak(_) => spans.push(Span::raw("\n")),
            _ => todo!(),
        }
    }
    spans.push(Span::raw("\n"));
    return spans;
}
