use std::{cmp::{self, max}, io::{stdout, Write}, path::{self, Path}, process::exit};

use crossterm::{cursor::{position, Hide, MoveDown, MoveLeft, MoveRight, MoveTo, MoveToNextLine, MoveToPreviousLine, MoveUp, RestorePosition, SavePosition, SetCursorStyle, Show}, event::{read, Event, KeyCode, KeyEvent}, execute, style::Print, terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType, SetSize}};

use crate::Buffer;
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
            Self::Insert    => print!("--INSERT MODE--"),
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

    pub fn cursor_home(&self) {
        let row = match self.file.length() {
            0 => 0,
            _ => self.file.length() - 1,
        };
            
        if let Some(last_line) = self.file.get_line(row) {
            let col = last_line.len();
            execute!(stdout(), MoveTo(col as u16, row as u16)).unwrap();
        }
    }

    pub fn render(&mut self) {
        let (col, row) = position().unwrap();

        disable_raw_mode().unwrap();
        execute!(stdout(), Hide, Clear(ClearType::All), MoveTo(0, 0)).unwrap();
        print!("\x1b[3J");
        self.file.print();

        // // Blank lines inbetween
        // if size().unwrap().1 > 2 {
        //     while position().unwrap().1 != size().unwrap().1 - 3 {
        //         execute!(stdout(), MoveToNextLine(1)).unwrap();
        //         print!("~");
        //     }
        // }
        
        self.set_mode(self.mode);
        execute!(stdout(), MoveTo(col, row), Show).unwrap();
        enable_raw_mode().unwrap();
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;

        let prev = position().unwrap();
        execute!(stdout(), MoveTo(0, size().unwrap().1)).unwrap();

        if mode == Mode::Normal {
            execute!(stdout(), Clear(ClearType::CurrentLine)).unwrap();
        } else {
            self.mode.print();
        }

        execute!(stdout(), MoveTo(prev.0, prev.1)).unwrap();
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
            match event {
                Event::Key(key_event) => {
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

                    self.set_mode(Mode::Normal);
                
                    // Reposition the cursor if it came out of command mode
                    if key_event.code == KeyCode::Char(':') {
                        execute!(stdout(), MoveTo(normal_pos.0, normal_pos.1)).unwrap();
                    }

                    enable_raw_mode().unwrap();
                },
                Event::Resize(col, row) => {
                    // print!("\x1b[3J");
                    // stdout().flush().unwrap();

                    self.render();
                },
                _ => {}
            }
        }
    }

    pub fn command(&mut self) {
        self.set_mode(Mode::Command);
        disable_raw_mode().unwrap();

        execute!(stdout(), MoveTo(0, size().unwrap().1 - 2), Clear(ClearType::CurrentLine), SavePosition).unwrap();
        print!(":");
        execute!(stdout(), RestorePosition, MoveRight(1)).unwrap();

        let mut command = String::new();
        std::io::stdin().read_line(&mut command).expect("Failed to read command");

        if command.trim() == "q" {
            execute!(stdout(), Clear(ClearType::Purge), Clear(ClearType::All), MoveTo(0, 0)).unwrap();
            print!("\x1b[3J");
            exit(0);
        } else if command.trim() == "s" {
            if let None = self.file.path {
                execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).unwrap();
                println!("Please enter a PATH to save file");
                print!("PATH: ");

                let mut path_str = String::new();
                std::io::stdin().read_line(&mut path_str).expect("Failed to read file name");
                self.file.path = Some(path_str);
            }
            self.file.write().expect("Failed to save file");
        }

        execute!(stdout(), MoveToPreviousLine(1), Clear(ClearType::CurrentLine)).unwrap();
    }

    pub fn process_backspace(&mut self, col: u16, row: u16) {
        if let Some(line) = self.file.get_line(row as usize) {
            if col == 0 {
                if row == 0 {
                    return;
                }

                // Join the previous line with the next line
                let mut prev_line = self.file.get_line(row as usize - 1).expect("Failed to get prev_line").clone();
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
                let first_half = (&line[0..col as usize]).iter().copied().collect::<Vec<char>>();
                let second_half = (&line[col as usize..line.len()]).iter().copied().collect::<Vec<char>>();

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