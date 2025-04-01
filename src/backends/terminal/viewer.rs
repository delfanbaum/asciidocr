use anyhow::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget, Wrap},
    DefaultTerminal, Frame,
};

use crate::graph::asg::Asg;

use super::components::line_from_block;

#[derive(Default, Debug)]
pub struct TermView {
    graph: Asg,
    exit: bool,
}

impl TermView {
    pub fn new(graph: Asg) -> Self {
        TermView { graph, exit: false }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            // up, down, j, k navigation, maybe a light/dark theme switcher?
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true
    }
}

impl Widget for &TermView {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let title = match &self.graph.source {
            Some(title) => Line::from(format!(" {title} ").bold()),
            None => Line::from(" STDIN "),
        };

        let instructions = Line::from(vec![
            " Scroll Up ".into(),
            "<k, up-arror>".blue().bold(),
            " Scroll down ".into(),
            "<j, down-arrow>".blue().bold(),
            " Quit ".into(),
            "<q> ".blue().bold(),
        ]);
        let container = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let mut contents: Vec<Line> = vec![];

        for block in self.graph.blocks.iter() {
            contents.push(line_from_block(block))
        }
        let text = Text::from(contents);

        Paragraph::new(text)
            .block(container)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }
}
