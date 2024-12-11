use anyhow::Result;
use ratatui::{DefaultTerminal, Frame};

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

    pub fn render(&self, frame: &mut Frame, asg: &Asg) {
        todo!()
    }

    fn handle_events(&mut self) -> Result<()> {
        todo!()
    }
}
