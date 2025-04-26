use std::{
    error::Error, fs::{self, File}, hint, io::{stdout, BufRead, BufReader, Write}, str::FromStr
};

use crossterm::{cursor::position, terminal::size};

use crate::utils;

#[derive(Clone)]
enum Edit {
    InsertChar  {row: usize, col: usize, c: char},
    DeleteChar  {row: usize, col: usize, deleted: char},
    SetLine     {row: usize, old_line: Vec<char>, new_line: Vec<char>},
    InsertLine  {row: usize, line: Vec<char>},
    DeleteLine  {row: usize, deleted: Vec<char>},
}

#[derive(Clone, Copy)]
pub enum Action {
    Do,
    Undo,
    Redo,
}

struct History {
    edits: Vec<Edit>,
    undos: Vec<Edit>
}

impl History {
    fn new() -> Self {
        Self { edits: vec![], undos: vec![]}
    }

    fn update(&mut self, edit: Edit, action: Action) {
        match action {
            Action::Do => self.edits.push(edit),
            Action::Undo => {
                if let Some(prev_edit) = self.edits.pop() {
                    self.undos.push(prev_edit);
                }
            },
            Action::Redo => {
                if let Some(prev_undo) = self.undos.pop() {
                    self.edits.push(prev_undo);
                }
            },
        } 
    }

    fn last_from(&self, action: Action) -> Option<Edit> {
        match action {
            Action::Do | Action::Redo => self.undos.last().cloned(),
            Action::Undo => self.edits.last().cloned(),
        }
    }
}


pub struct Buffer {
    pub path: Option<String>,
    pub modified: bool,

    data: Vec<Vec<char>>,
    pub start: usize,

    history: History,
}

impl FromStr for Buffer {
    type Err = Box<dyn Error>;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut buffer_data = Vec::new();
        for line_result in reader.lines() {
            let line = line_result.unwrap().chars().collect::<Vec<char>>();
            buffer_data.push(line);
        }

        // Start the buffer off with a single empty line if the opened file has no lines
        if buffer_data.is_empty() {
            buffer_data.push(vec![]);
        }

        Ok(Buffer::from(path.to_string(), buffer_data))
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self { 
            modified: false,
            start: 0,
            path: None,
            data: vec![vec![]],
            history: History::new(),
        }
    }
}

impl Buffer {
    pub fn new(path: Option<String>) -> Self {
        let mut new_buffer = Buffer::default();
        new_buffer.path = path;
        new_buffer
    }

    fn from(path: String, data: Vec<Vec<char>>) -> Self {
        let mut new_buffer = Buffer::default();
        new_buffer.path = Some(path);
        new_buffer.data = data;
        new_buffer
    }

    pub fn move_down(&mut self) -> bool {
        if self.length() != 0 {
            if self.start + (position().unwrap().1 as usize) < self.length() - 1 {
                self.start += 1;
                return true;
            }
        }
        return false;
    }

    pub fn move_up(&mut self) -> bool {
        if self.length() != 0 {
            if self.start > 0 {
                self.start -= 1;
                return true;
            }
        }
        return false;
    }
    

    pub fn length(&self) -> usize {
        self.data.len()
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (i, line) in self.data.iter().enumerate() {
            for char in line {
                bytes.push(*char as u8)
            }

            if i != self.data.len() {
                bytes.push(b'\n');
            }
        }
        bytes
    }

    pub fn write(&self) -> Result<(), std::io::Error> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Path is empty"))?;

        fs::write(path, self.bytes())
    }

    pub fn print(&self) {
        for i in self.start..(self.start + utils::window_size() as usize + 1) {
            if let Some(line) = self.data.get(i) {
                for char in line {
                    print!("{}", char);
                }
            }

            if i != self.data.len() - 1 {
                println!();
            }
        }

        // Make sure all print!() output shows up before exiting
        stdout().flush().unwrap();
    }

    pub fn get_line(&self, line: usize) -> Option<&Vec<char>> {
        self.data.get(line + self.start)
    }

    pub fn set_line(&mut self, line: usize, new_line: Vec<char>, action: Action) {
        if let Some(old_line) = self.data.get_mut(line + self.start) {
            self.modified = true;
            let edit = Edit::SetLine { row: line, old_line: old_line.clone(), new_line: new_line.clone() };
            self.history.update(edit, action);

            *old_line = new_line;
            self.modified = true;
        }
    }

    pub fn insert_line(&mut self, row: usize, line: Vec<char>, action: Action) {
        if row < self.data.len() {
            self.history.update(Edit::InsertLine { row: row + self.start, line: line.clone() }, action);
            self.data.insert(row + self.start, line);
        } else {
            self.history.update(Edit::InsertLine { row, line: line.clone() }, action);
            self.data.push(line);
        }

        self.modified = true;
    }

    pub fn get_line_mut(&mut self, line: usize) -> Option<&mut Vec<char>> {
        self.data.get_mut(line + self.start)
    }

    pub fn get_char(&self, line: usize, col: usize) -> Option<&char> {
        self.data.get(line + self.start).and_then(|l| l.get(col))
    }

    pub fn delete_char(&mut self, row: usize, col: usize, action: Action) {
        if let Some(line) = self.data.get_mut(row + self.start) {
            if col < line.len() {
                let deleted = line.remove(col);

                self.modified = true;
                let edit = Edit::DeleteChar { row, col, deleted };
                self.history.update(edit, action);
            }
        }
    }

    pub fn insert_char(&mut self, row: usize, col: usize, c: char, action: Action) {
        if let Some(line) = self.data.get_mut(row + self.start) {
            if col > line.len() {
                line.push(c);
            } else {
                line.insert(col, c);
            }

            self.modified = true;
            let edit = Edit::InsertChar { row, col, c };
            self.history.update(edit, action);
        }
    }

    pub fn delete_line(&mut self, row: usize, action: Action) {
        if row < self.data.len() {
            let deleted = self.data.get(row).expect("Deleted line does not exist");
            
            self.history.update(Edit::DeleteLine { row: row + self.start, deleted: deleted.clone()}, action);
            self.data.remove(row + self.start);
            self.modified = true;
        }
    }

    pub fn undo(&mut self) {
        if let Some(edit) = self.history.last_from(Action::Undo) {
            match edit {
                Edit::InsertChar { row, col, c: _ }                      => self.delete_char(row, col, Action::Undo),
                Edit::DeleteChar { row, col, deleted }             => self.insert_char(row, col,deleted, Action::Undo),
                Edit::SetLine    { row, old_line, new_line: _ }      => self.set_line(row, old_line, Action::Undo),
                Edit::InsertLine { row, line: _ }                               => self.delete_line(row, Action::Undo),
                Edit::DeleteLine { row, deleted }                    => self.insert_line(row, deleted, Action::Undo),
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some(edit) = self.history.last_from(Action::Redo) {
            match edit {
                Edit::InsertChar { row, col, c }                  => self.insert_char(row, col, c, Action::Redo),
                Edit::DeleteChar { row, col, deleted: _ }               => self.delete_char(row, col, Action::Redo),
                Edit::SetLine    { row, old_line: _, new_line }     => self.set_line(row, new_line, Action::Redo),
                Edit::InsertLine { row, line }                      => self.insert_line(row, line, Action::Redo),
                Edit::DeleteLine { row, deleted: _ }                           => self.delete_line(row, Action::Redo),
            }
        }
    }
}
