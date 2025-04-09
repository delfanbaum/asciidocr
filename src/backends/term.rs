use crate::graph::asg::Asg;
use crate::graph::blocks::Block;
use crate::graph::inlines::{Inline, InlineSpanVariant};
use anyhow::Result;
use std::io::Write;
use termcolor::{Buffer, BufferWriter, Color, ColorSpec, WriteColor};

pub struct TermRenderer {
    writer: BufferWriter,
    buffer: Buffer,
    line_length: usize,
    count: usize,
}

impl Default for TermRenderer {
    fn default() -> Self {
        Self::new(80)
    }
}

impl TermRenderer {
    pub fn new(line_length: usize) -> Self {
        let writer = BufferWriter::stdout(termcolor::ColorChoice::Auto);
        let buffer = writer.buffer();
        Self {
            writer,
            buffer,
            line_length,
            count: 0,
        }
    }

    pub fn render_to_term(&mut self, graph: &Asg) -> Result<()> {
        for block in graph.blocks.iter() {
            self.render_block(block)?;
            self.buffer.reset()?;
        }
        Ok(self.writer.print(&self.buffer)?)
    }

    fn bufwrite(&mut self, contents: &str) -> Result<()> {
        // TODO... count the words and
        Ok(write!(self.buffer, "{}", contents)?)
    }

    fn render_block(&mut self, block: &Block) -> Result<()> {
        for inline in block.inlines() {
            self.render_inline(inline, true)?
        }

        // newlines after block
        self.bufwrite("\n\n")?;
        Ok(())
    }

    fn render_inline(&mut self, inline: &Inline, reset: bool) -> Result<()> {
        match inline {
            Inline::InlineLiteral(literal) => {
                self.bufwrite(&literal.string_repr())?;
            }
            Inline::InlineSpan(ref span) => match span.variant {
                InlineSpanVariant::Strong => {
                    self.buffer.set_color(ColorSpec::new().set_bold(true))?;
                    for inline in span.inlines.iter() {
                        self.render_inline(inline, false)?
                    }
                }
                InlineSpanVariant::Emphasis => {
                    self.buffer.set_color(ColorSpec::new().set_italic(true))?;
                    for inline in span.inlines.iter() {
                        self.render_inline(inline, false)?
                    }
                }
                InlineSpanVariant::Mark => {
                    self.buffer
                        .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
                    for inline in span.inlines.iter() {
                        self.render_inline(inline, false)?
                    }
                }
                InlineSpanVariant::Code => {
                    self.buffer
                        .set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
                    for inline in span.inlines.iter() {
                        self.render_inline(inline, false)?
                    }
                }
                InlineSpanVariant::Superscript | InlineSpanVariant::Subscript => {
                    self.buffer
                        .set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
                    for inline in span.inlines.iter() {
                        self.render_inline(inline, false)?
                    }
                }

                InlineSpanVariant::Footnote => {
                    self.buffer
                        .set_color(ColorSpec::new().set_fg(Some(Color::Magenta)))?;
                    self.bufwrite("[")?;
                    for inline in span.inlines.iter() {
                        self.render_inline(inline, false)?
                    }
                    self.bufwrite("]")?;
                }
            },
            Inline::InlineBreak(_) => {
                self.bufwrite("\n")?;
            }
            _ => {}
        }
        if reset {
            self.buffer.reset()?;
        }
        Ok(())
    }
}
