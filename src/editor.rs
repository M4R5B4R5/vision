use std::{
    cmp::{self, max}, collections::VecDeque, io::{stdout, Write}, path::{self, Path}, process::exit
};

use crossterm::{
    cursor::{
        position, Hide, MoveDown, MoveLeft, MoveRight, MoveTo, MoveToNextLine, MoveToPreviousLine, MoveUp, RestorePosition, SavePosition, SetCursorStyle, Show
    },
    event::{read, Event, KeyCode, KeyEvent},
    execute,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, size, window_size, Clear, ClearType, SetSize},
};
use crossterm::style::Color;

use crate::{mode::*, print_bg, print_fg, utils, Buffer, Command, Cursor, CursorPosition, History, Redo, Undo};
use cmp::min;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub fn add(&self, pos: (u16, u16)) -> (u16, u16) {
        match self {
            Direction::Up => (pos.0, pos.1 - 1),
            Direction::Down => (pos.0, pos.1 + 1),
            Direction::Left => (pos.0 - 1, pos.1),
            Direction::Right => (pos.0 + 1, pos.1),
        }
    }
}

pub struct Editor {
    pub file: Buffer,
    pub cursor: Cursor,
    pub mode: Mode,

    prev_cursor_col: Option<u16>,
}

impl Undo for Editor {
    fn undo(&mut self) {
        self.file.undo();
        self.cursor.undo();
    }
}

impl Redo for Editor {
    fn redo(&mut self) {
        self.file.redo();
        self.cursor.redo();
    }
}

impl Editor {
    pub fn new(file: Buffer) -> Self {
        execute!(
            stdout(),
            Clear(ClearType::All),
            MoveTo(0, 0),
            SetCursorStyle::SteadyBar,
            Show
        )
        .unwrap();
        print!("\x1b[3J");
        Self {
            file,
            cursor: Cursor::new(History::<CursorPosition>::new()),
            mode: Mode::Normal,
            prev_cursor_col: None,
        }
    }

    pub fn cursor_home(&self) {
        let mut row = min(self.file.length(), utils::window_size() as usize);
        if row == self.file.length() && row != 0 {
            row -= 1;
        } 

        if let Some(last_line) = self.file.get_line(row) {
            let col = last_line.len();
            execute!(stdout(), MoveTo(col as u16, row as u16)).unwrap();
        }
    }

    pub fn cursor_command(&self) {
        let row = size().unwrap().1;
        if row > 0 {
            execute!(stdout(), MoveTo(0, row - 1)).unwrap();
        }
    }

    pub fn render(&mut self) {
        let (col, row) = position().unwrap();

        disable_raw_mode().unwrap();
        execute!(stdout(), Hide, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
        print!("\x1b[3J");
        self.file.print();

        self.set_mode(self.mode);
        execute!(stdout(), MoveTo(col, row), Show).unwrap();
        enable_raw_mode().unwrap();
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;

        let prev = position().unwrap();
        execute!(stdout(), MoveTo(0, utils::window_size() + 1)).unwrap();

        self.mode.get().print();

        execute!(stdout(), MoveTo(prev.0, prev.1)).unwrap();
    }

    pub fn listen(&mut self) {
        self.mode.get().listen(self);
    }

    fn move_cursor_up(&mut self, mut cur_pos: (u16, u16)) -> Option<()> {
        if cur_pos.1 <= 0 {
            if self.file.move_up() {
                execute!(stdout(), MoveDown(1)).unwrap();
                cur_pos = position().unwrap();
                self.render();
            } else {
                return None;
            }
        }

        let up_pos = (cur_pos.0, cur_pos.1 - 1);
        let next_line = self.file.get_line(up_pos.1 as usize)?;

        if let Some(_) = next_line.get(up_pos.0 as usize) {
            if let Some(prev_cursor_col) = self.prev_cursor_col {
                execute!(
                    stdout(),
                    MoveToPreviousLine(1),
                    MoveRight(min(prev_cursor_col, next_line.len() as u16))
                )
                .unwrap();
                self.prev_cursor_col = None;
            } else {
                execute!(stdout(), MoveUp(1)).unwrap();
            }
        } else if next_line.is_empty() {
            self.prev_cursor_col = Some(cur_pos.0);
            execute!(stdout(), MoveToPreviousLine(1)).unwrap();
        } else {
            execute!(
                stdout(),
                MoveToPreviousLine(1),
                MoveRight(next_line.len() as u16)
            )
            .unwrap();
        }

        Some(())
    }

    pub fn move_cursor_down(&mut self, mut cur_pos: (u16, u16)) -> Option<()> {
        if cur_pos.1 >= utils::window_size() {
            if self.file.move_down() {
                execute!(stdout(), MoveUp(1)).unwrap();
                cur_pos = position().unwrap();
                self.render();
            }
        }

        let down_pos = (cur_pos.0, cur_pos.1 + 1);
        let next_line = self.file.get_line(down_pos.1 as usize)?;

        if let Some(_) = next_line.get(down_pos.0 as usize) {
            if let Some(prev_cursor_col) = self.prev_cursor_col {
                execute!(
                    stdout(),
                    MoveToNextLine(1),
                    MoveRight(min(prev_cursor_col, next_line.len() as u16))
                )
                .unwrap();
                self.prev_cursor_col = None;
            } else {
                execute!(stdout(), MoveDown(1)).unwrap();
            }
        } else if next_line.is_empty() {
            self.prev_cursor_col = Some(cur_pos.0);
            execute!(stdout(), MoveToNextLine(1)).unwrap();
        } else {
            execute!(
                stdout(),
                MoveToNextLine(1),
                MoveRight(next_line.len() as u16)
            )
            .unwrap();
        }
        Some(())
    }

    fn move_cursor_left(&mut self, cur_pos: (u16, u16)) -> Option<()> {
        if cur_pos.0 <= 0 {
            return None;
        }

        let left_pos = (cur_pos.0 - 1, cur_pos.1);
        self.file.get_line(left_pos.1 as usize)?;

        execute!(stdout(), MoveLeft(1)).unwrap();
        Some(())
    }

    fn move_cursor_right(&mut self, cur_pos: (u16, u16)) -> Option<()> {
        let right_pos = (cur_pos.0 + 1, cur_pos.1);
        let current_line = self.file.get_line(right_pos.1 as usize)?;

        if right_pos.0 <= current_line.len() as u16 {
            execute!(stdout(), MoveRight(1)).unwrap();
        }
        Some(())
    }

    pub fn move_cursor(&mut self, dir: Direction) -> Option<()> {
        let cur_pos = position().unwrap();

        match dir {
            Direction::Left => self.move_cursor_left(cur_pos),
            Direction::Right => self.move_cursor_right(cur_pos),
            Direction::Up => self.move_cursor_up(cur_pos),
            Direction::Down => self.move_cursor_down(cur_pos),
        }
    }
}