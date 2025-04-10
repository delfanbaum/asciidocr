use crate::graph::asg::Asg;
use crate::graph::blocks::{Block, BreakVariant, ParentBlockName};
use crate::graph::inlines::{Inline, InlineSpanVariant};
use anyhow::Result;
use std::io::Write;
use termcolor::{Buffer, BufferWriter, Color, ColorSpec, WriteColor};

pub struct TermRenderer {
    writer: BufferWriter,
    buffer: Buffer,
    line_length: usize,
    count: usize,  // eventually will determine when to break a line
    indent: usize, // holds info for line indentation across breaks
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
            indent: 0,
        }
    }

    pub fn render_to_term(&mut self, graph: &Asg) -> Result<()> {
        // clear screen
        print!("\x1B[2J\x1B[1;1H");
        if let Some(header) = &graph.header {
            self.buffer
                .set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))?;
            self.bufwrite("= ")?;
            for inline in header.title.iter() {
                self.render_inline(inline, false)?;
            }
            self.bufwrite("\n\n")?;
            self.buffer.reset()?;
        }
        for block in graph.blocks.iter() {
            self.render_block(block, true)?;
            self.buffer.reset()?;
        }
        Ok(self.writer.print(&self.buffer)?)
    }

    fn bufwrite(&mut self, contents: &str) -> Result<()> {
        if self.count >= self.line_length {
            // note that this won't occur
            todo!()
        }
        Ok(write!(self.buffer, "{}", contents)?)
    }

    fn _break_line(&mut self) -> Result<()> {
        self.count = 0;
        Ok(self.bufwrite(&format!("\n{:indent$}", "", indent = self.indent))?)
    }

    fn render_block(&mut self, block: &Block, mut space_after: bool) -> Result<()> {
        match block {
            Block::Section(section) => {
                self.buffer
                    .set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))?;
                let level = "=".repeat(section.level);
                self.bufwrite(&format!("{} ", level))?;
                for inline in &section.title() {
                    self.render_inline(inline, false)?;
                }
                self.buffer.reset()?;

                for block in section.blocks.iter() {
                    self.render_block(block, true)?;
                }
            }

            Block::List(list) => {
                for item in list.items.iter() {
                    self.bufwrite(&format!("{} ", list.marker))?;
                    self.render_block(item, false)?;
                }
                space_after = false; // to avoid additional extra space
            }
            Block::ListItem(li) => {
                for inline in li.principal.iter() {
                    self.render_inline(inline, true)?;
                }
                if li.has_blocks() {
                    self.bufwrite("\n")?;
                    self.indent = 2; // add indentation
                    self.count = 2; // start the count at 2
                    for block in li.blocks.iter() {
                        self.render_block(block, true)?;
                    }
                    self.indent = 0; // add indentation
                }
            }
            Block::DList(list) => {
                for dli in list.items.iter() {
                    self.render_block(dli, false)?
                }
                space_after = false; // to avoid additional extra space
            }
            Block::DListItem(dli) => {
                // set term color
                self.buffer
                    .set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))?;
                for inline in dli.terms.iter() {
                    self.render_inline(inline, false)?;
                }
                self.bufwrite(":: ")?;
                // clear term color
                self.buffer.reset()?;
                for inline in dli.principal.iter() {
                    self.render_inline(inline, false)?;
                }
                if dli.has_blocks() {
                    for block in dli.blocks.iter() {
                        self.render_block(block, true)?;
                    }
                }
            }
            Block::ParentBlock(parent) => {
                if let Some(variant) = &parent.variant {
                    self.buffer
                        .set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(true))?;
                    self.bufwrite(&format!("{}: ", variant))?;
                    self.buffer.reset()?;
                }

                if !parent.title.is_empty() {
                    self.buffer
                        .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
                    for inline in parent.title.iter() {
                        self.render_inline(inline, false)?;
                    }
                    self.buffer.reset()?;
                    self.bufwrite("\n")?;
                }

                self.indent = 2;
                for block in parent.blocks.iter() {
                    let space_after = matches!(parent.name, ParentBlockName::Sidebar);
                    self.render_block(block, space_after)?;
                }
                self.indent = 0;
                space_after = false; // to avoid additional extra space
            }
            Block::Break(br) => match br.variant {
                BreakVariant::Page => {
                    self.buffer
                        .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
                    self.bufwrite("<<<")?;
                    self.buffer.reset()?;
                }
                BreakVariant::Thematic => {
                    self.buffer
                        .set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
                    self.bufwrite("'''")?;
                    self.buffer.reset()?;
                }
            },
            _ => {
                for inline in block.inlines() {
                    self.render_inline(inline, true)?
                }
            }
        }

        // newlines after block
        if space_after {
            self.bufwrite("\n\n")?;
        } else {
            self.bufwrite("\n")?;
        }
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
