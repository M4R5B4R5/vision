use std::{cmp, io::stdout, process::exit};

use crossterm::{cursor::{position, MoveDown, MoveLeft, MoveRight, MoveTo, MoveToNextLine, MoveToPreviousLine, MoveUp, RestorePosition, SavePosition, SetCursorStyle, Show}, event::{read, KeyCode}, execute, terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType}};

use crate::Buffer;
use cmp::min;

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Normal,
    Insert,
    Command,
}

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
            Direction::Up       => (pos.0, pos.1 - 1),
            Direction::Down     => (pos.0, pos.1 + 1),
            Direction::Left     => (pos.0 - 1, pos.1),
            Direction::Right    => (pos.0 + 1, pos.1),
        }
    }
}

pub struct Editor {
    file: Buffer,
    mode: Mode,

    prev_cursor_col: Option<u16>,
}

impl Editor {
    pub fn new(file: Buffer) -> Self {
        execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0), SetCursorStyle::SteadyBar, Show).unwrap();
        print!("\x1b[3J");
        Self { file, mode: Mode::Normal, prev_cursor_col: None }
    }

    pub fn render(&self) {
        self.file.print();
    }

    pub fn listen(&mut self) {
        match self.mode {
            Mode::Normal => self.normal(),
            Mode::Insert => self.insert(),
            Mode::Command => self.command(),
        }
    }

    fn move_cursor_up(&mut self, cur_pos: (u16, u16)) -> Option<()> {
        if cur_pos.1 <= 0 {
            return None;
        }

        let up_pos = (cur_pos.0, cur_pos.1 - 1);
        let next_line = self.file.get_line(up_pos.1 as usize)?;

        if let Some(_) = next_line.get(up_pos.0 as usize) {
            if let Some(prev_cursor_col) = self.prev_cursor_col {
                execute!(stdout(), MoveToPreviousLine(1), MoveRight(min(prev_cursor_col, next_line.len() as u16))).unwrap();
                self.prev_cursor_col = None;
            } else {
                execute!(stdout(), MoveUp(1)).unwrap();
            }
        } else if next_line.is_empty() {
            self.prev_cursor_col = Some(cur_pos.0);
            execute!(stdout(), MoveToPreviousLine(1)).unwrap();
        } else {
            execute!(stdout(), MoveToPreviousLine(1), MoveRight(next_line.len() as u16)).unwrap();
        }

        Some(())
    }

    fn move_cursor_down(&mut self, cur_pos: (u16, u16)) -> Option<()> {
        let down_pos = (cur_pos.0, cur_pos.1 + 1);
        let next_line = self.file.get_line(down_pos.1 as usize)?;

        if let Some(_) = next_line.get(down_pos.0 as usize) {
            if let Some(prev_cursor_col) = self.prev_cursor_col {
                execute!(stdout(), MoveToNextLine(1), MoveRight(min(prev_cursor_col, next_line.len() as u16))).unwrap();
                self.prev_cursor_col = None;
            } else {
                execute!(stdout(), MoveDown(1)).unwrap();
            }
        } else if next_line.is_empty() {
            self.prev_cursor_col = Some(cur_pos.0);
            execute!(stdout(), MoveToNextLine(1)).unwrap();
        } else {
            execute!(stdout(), MoveToNextLine(1), MoveRight(next_line.len() as u16)).unwrap();
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

    pub fn move_cursor(&mut self, dir: Direction) -> Option<()>{
        let cur_pos = position().unwrap();
        
        match dir {
            Direction::Left => self.move_cursor_left(cur_pos),
            Direction::Right => self.move_cursor_right(cur_pos),
            Direction::Up => self.move_cursor_up(cur_pos),
            Direction::Down => self.move_cursor_down(cur_pos),
        }
    }

    pub fn normal(&mut self) {
        enable_raw_mode().unwrap();
        while let Ok(event) = read() {
            if let Some(key_event) = event.as_key_event() {
                let normal_pos = position().unwrap();

                match key_event.code {
                    // Other mode listeners
                    KeyCode::Char(':') => self.command(),
                    KeyCode::Char('i') => self.insert(),

                    // Cursor movement
                    KeyCode::Char('h') => {self.move_cursor(Direction::Left);},
                    KeyCode::Char('k') => {self.move_cursor(Direction::Up);},
                    KeyCode::Char('l') => {self.move_cursor(Direction::Right);},
                    KeyCode::Char('j') => {self.move_cursor(Direction::Down);},

                    // Shortcuts comming soon                    
                    _ => {}
                }

                // Reposition the cursor if it came out of another mode
                if key_event.code != KeyCode::Char('h') && key_event.code != KeyCode::Char('k') && key_event.code != KeyCode::Char('l') && key_event.code != KeyCode::Char('j') {
                    execute!(stdout(), MoveTo(normal_pos.0, normal_pos.1)).unwrap();
                }
                enable_raw_mode().unwrap();
            }
        }
    }

    pub fn command(&self) {
        disable_raw_mode().unwrap();

        execute!(stdout(), MoveTo(0, self.file.length() as u16 + 1), SavePosition).unwrap();
        print!(":");
        execute!(stdout(), RestorePosition, MoveRight(1)).unwrap();

        let mut command = String::new();
        std::io::stdin().read_line(&mut command).expect("Failed to read command");

        if command.trim() == "q" {
            execute!(stdout(), Clear(ClearType::Purge), Clear(ClearType::All), MoveTo(0, 0)).unwrap();
            print!("\x1b[3J");
            exit(0);
        }

        execute!(stdout(), MoveToPreviousLine(1), Clear(ClearType::CurrentLine)).unwrap();
    }
    
    pub fn insert(&self) {
        disable_raw_mode().unwrap();
    }
}