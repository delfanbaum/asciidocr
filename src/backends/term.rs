use crate::graph::asg::Asg;
use crate::graph::blocks::Block;
use crate::graph::inlines::{Inline, InlineSpanVariant};
use anyhow::Result;
use std::io::Write;
use termcolor::{Buffer, BufferWriter, Color, ColorSpec, WriteColor};

pub fn render_to_term(graph: &Asg) -> Result<()> {
    let bufwriter = BufferWriter::stdout(termcolor::ColorChoice::Auto);
    let mut buffer = bufwriter.buffer();

    for block in graph.blocks.iter() {
        render_block(block, &mut buffer, &bufwriter)?;
        buffer.reset()?;
    }

    Ok(())
}

fn render_block(block: &Block, buffer: &mut Buffer, writer: &BufferWriter) -> Result<()> {
    for inline in block.inlines() {
        render_inline(inline, buffer, writer, true)?
    }

    // newlines after block
    write!(buffer, "\n\n")?;
    writer.print(&buffer)?;
    Ok(())
}

fn render_inline(
    inline: &Inline,
    buffer: &mut Buffer,
    writer: &BufferWriter,
    reset: bool,
) -> Result<()> {
    match inline {
        Inline::InlineLiteral(literal) => {
            write!(buffer, "{}", literal.string_repr())?;
            writer.print(&buffer)?;
        }
        Inline::InlineSpan(ref span) => match span.variant {
            InlineSpanVariant::Strong => {
                buffer.set_color(&ColorSpec::new().set_bold(true))?;
                for inline in span.inlines.iter() {
                    render_inline(inline, buffer, writer, false)?;
                }
            }
            InlineSpanVariant::Emphasis => {
                buffer.set_color(&ColorSpec::new().set_italic(true))?;
                for inline in span.inlines.iter() {
                    render_inline(inline, buffer, writer, false)?;
                }
            }
            InlineSpanVariant::Mark => {
                buffer.set_color(&ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                for inline in span.inlines.iter() {
                    render_inline(inline, buffer, writer, false)?;
                }
            }
            InlineSpanVariant::Code => {
                buffer.set_color(&ColorSpec::new().set_fg(Some(Color::Red)))?;
                for inline in span.inlines.iter() {
                    render_inline(inline, buffer, writer, false)?;
                }
            }
            InlineSpanVariant::Superscript | InlineSpanVariant::Subscript => {
                buffer.set_color(&ColorSpec::new().set_fg(Some(Color::Blue)))?;
                for inline in span.inlines.iter() {
                    render_inline(inline, buffer, writer, false)?;
                }
            }

            InlineSpanVariant::Footnote => {
                buffer.set_color(&ColorSpec::new().set_fg(Some(Color::Magenta)))?;
                for inline in span.inlines.iter() {
                    render_inline(inline, buffer, writer, false)?;
                }
            }
        },
        Inline::InlineBreak(_) => {
            write!(buffer, "\n")?;
            writer.print(&buffer)?;
        }
        _ => {}
    }
    buffer.clear();
    if reset {
        buffer.reset()?;
    }
    Ok(())
}
