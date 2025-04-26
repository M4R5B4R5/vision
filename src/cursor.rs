use std::io::stdout;

use crossterm::{cursor::{position, MoveTo}, execute};

use crate::{Action, History, Redo, Undo};

#[derive(Clone, Copy)]
pub struct CursorPosition {
    old: (u16, u16),
    new: (u16, u16),
}

impl CursorPosition {
    pub fn new(old: (u16, u16), new: (u16, u16)) -> Self {
        Self {old, new}
    }
}

pub struct Cursor {
    pub history: History<CursorPosition>
}

impl Cursor {
    pub fn new(history: History<CursorPosition>) -> Self {
        Self { history }
    }

    pub fn pos() -> (u16, u16) {
        position().unwrap()
    }

    pub fn move_to(pos: (u16, u16)) {
        execute!(stdout(), MoveTo(pos.0, pos.1)).unwrap();
    }
}

impl Undo for Cursor {
    fn undo(&mut self) {
        if let Some(prev_position) = self.history.last_from(Action::Undo) {
            Cursor::move_to(prev_position.old);
            self.history.update(prev_position, Action::Undo);
        }
    }
}

impl Redo for Cursor {
    fn redo(&mut self) {
        if let Some(next_position) = self.history.last_from(Action::Redo) {
            Cursor::move_to(next_position.new);
            self.history.update(next_position, Action::Redo);
        }
    }
}