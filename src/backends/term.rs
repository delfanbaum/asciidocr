use anyhow::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};

use crate::graph::asg::Asg;

#[derive(Default, Debug)]
pub struct TermView {
    exit: bool,
}

impl TermView {
    pub fn run(&mut self, terminal: &mut DefaultTerminal, asg: Asg) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render(frame, &asg))?;
            self.handle_events()?;
        }
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame, _asg: &Asg) {
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
        let title = Line::from(" Counter App Tutorial ".bold());
        let instructions = Line::from(vec![
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let counter_text = "foo";

        Paragraph::new(counter_text)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
