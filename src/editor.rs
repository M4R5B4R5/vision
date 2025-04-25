use std::{
    cmp::{self, max},
    io::{Write, stdout},
    path::{self, Path},
    process::exit,
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

use crate::{print_fg, print_bg, utils, Buffer, Command};
use cmp::min;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Command,
}

impl Mode {
    fn print(&self) {
        match self {
            Self::Insert    => print_fg!(Color::Yellow, "-=INSERT MODE=-"),
            Self::Normal    => print_fg!(Color::Green, "-=NORMAL MODE=-"),
            Self::Command   => print_fg!(Color::Magenta, "-=COMMAND MODE=-"),
            _ => {}
        }
        stdout().flush().unwrap();
    }
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
            Direction::Up => (pos.0, pos.1 - 1),
            Direction::Down => (pos.0, pos.1 + 1),
            Direction::Left => (pos.0 - 1, pos.1),
            Direction::Right => (pos.0 + 1, pos.1),
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

        self.mode.print();

        execute!(stdout(), MoveTo(prev.0, prev.1)).unwrap();
    }

    pub fn listen(&mut self) {
        match self.mode {
            Mode::Normal => self.normal(),
            Mode::Insert => self.insert(),
            Mode::Command => self.command(),
        }
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

    fn move_cursor_down(&mut self, mut cur_pos: (u16, u16)) -> Option<()> {
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

    pub fn normal(&mut self) {
        enable_raw_mode().unwrap();
        while let Ok(event) = read() {
            match event {
                Event::Key(key_event) => {
                    let normal_pos = position().unwrap();
                    match key_event.code {
                        // Other mode listeners
                        KeyCode::Char(':') => self.command(),
                        KeyCode::Char('i') => self.insert(),

                        // Cursor movement
                        KeyCode::Char('h') => {
                            self.move_cursor(Direction::Left);
                        }
                        KeyCode::Char('k') => {
                            self.move_cursor(Direction::Up);
                        }
                        KeyCode::Char('l') => {
                            self.move_cursor(Direction::Right);
                        }
                        KeyCode::Char('j') => {
                            self.move_cursor(Direction::Down);
                        }

                        // Shortcuts comming soon
                        _ => {}
                    }

                    self.set_mode(Mode::Normal);

                    // Reposition the cursor if it came out of command mode
                    if key_event.code == KeyCode::Char(':') {
                        execute!(stdout(), MoveTo(normal_pos.0, normal_pos.1)).unwrap();
                    }

                    enable_raw_mode().unwrap();
                }
                Event::Resize(col, row) => {
                    self.render();
                }
                _ => {}
            }
        }
    }

    pub fn command(&mut self) {
        self.set_mode(Mode::Command);
        self.cursor_command();

        utils::clear_line();

        print_fg!(Color::DarkYellow, ":");
        stdout().flush().unwrap();

        let mut command_str = String::new();

        while let Some(key_event) = read().unwrap().as_key_event() {
            match key_event.code {
                KeyCode::Char(c) => {
                    print_fg!(Color::DarkYellow, "{}", c);
                    stdout().flush().unwrap();
                    command_str.push(c);
                }
                KeyCode::Backspace => match command_str.pop() {
                    Some(_) => {
                        execute!(stdout(), MoveLeft(1), Clear(ClearType::UntilNewLine)).unwrap()
                    }
                    None => {
                        utils::clear_line();
                        break
                    },
                },
                KeyCode::Enter => {
                    utils::clear_line();
                    match command_str.parse::<Command>() {
                        Ok(command) => {
                            if let Err(e) = command.run(&self.file) {
                                utils::clear_line();
                                print_bg!(Color::DarkRed, "{:?} - PRESS ANY KEY TO CONTINUE", e);
                                execute!(stdout(), MoveToPreviousLine(1), MoveDown(1)).unwrap();

                                // Press any key to continue
                                read().unwrap();
                            } 
                        }
                        Err(e) => {
                            utils::clear_line();
                            print_bg!(Color::DarkRed, "{:?} - PRESS ANY KEY TO CONTINUE", e);

                            // Press any key to continue
                            read().unwrap();
                        },
                    }
                    execute!(stdout(), Clear(ClearType::CurrentLine)).unwrap();
                    break;
                }
                _ => {}
            }
        }
    }

    pub fn process_backspace(&mut self, col: u16, row: u16) {
        if let Some(line) = self.file.get_line(row as usize) {
            if col == 0 {
                if row == 0 {
                    return;
                }

                // Join the previous line with the next line
                let mut prev_line = self
                    .file
                    .get_line(row as usize - 1)
                    .expect("Failed to get prev_line")
                    .clone();
                let prev_line_len = prev_line.len();
                prev_line.extend(line);
                self.file.set_line(row as usize - 1, prev_line);

                execute!(stdout(), MoveUp(1)).unwrap();

                // Move the cursor to end of line if previous line isn't empty
                if prev_line_len > 0 {
                    execute!(stdout(), MoveRight(prev_line_len as u16)).unwrap();
                }

                // Delete current line and move cursor up
                self.file.delete_line(row as usize);
            } else {
                self.file.delete_char(row as usize, (col - 1) as usize);
                execute!(stdout(), MoveLeft(1)).unwrap();
            }
        }
    }

    pub fn process_enter(&mut self, col: u16, row: u16) {
        if let Some(line) = self.file.get_line(row as usize) {
            if col < line.len() as u16 {
                let first_half = (&line[0..col as usize])
                    .iter()
                    .copied()
                    .collect::<Vec<char>>();
                let second_half = (&line[col as usize..line.len()])
                    .iter()
                    .copied()
                    .collect::<Vec<char>>();

                self.file.set_line(row as usize, first_half);
                self.file.insert_line(row as usize + 1, second_half);

                execute!(stdout(), MoveToNextLine(1)).unwrap();
            } else {
                self.file.insert_line(row as usize + 1, Vec::new());
                self.move_cursor_down((col, row));
            }
        }
    }

    pub fn process_char(&mut self, col: u16, row: u16, c: char) {
        self.file.insert_char(row as usize, col as usize, c);
        execute!(stdout(), MoveRight(1)).unwrap();
    }

    pub fn insert(&mut self) {
        self.set_mode(Mode::Insert);
        while let Ok(event) = read() {
            if let Some(key_event) = event.as_key_event() {
                let (col, row) = position().unwrap();
                match key_event.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => self.process_enter(col, row),
                    KeyCode::Backspace => self.process_backspace(col, row),
                    KeyCode::Char(c) => self.process_char(col, row, c),
                    _ => {}
                }
                self.render();
            }
        }
    }
}
