use std::{
    error::Error,
    fs::{self, File},
    io::{BufRead, BufReader, Write, stdout},
    str::FromStr,
};

use crossterm::{cursor::position, terminal::size};

use crate::utils;

pub struct Buffer {
    pub path: Option<String>,
    pub modified: bool,

    data: Vec<Vec<char>>,

    pub start: usize,
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

impl Buffer {
    pub fn new(path: Option<String>) -> Self {
        Self {
            modified: false,
            start: 0,
            path,
            data: vec![vec![]],
        }
    }

    fn from(path: String, data: Vec<Vec<char>>) -> Self {
        Self {
            modified: false,
            start: 0,
            path: Some(path),
            data,
        }
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

    pub fn set_line(&mut self, line: usize, new_line: Vec<char>) {
        if let Some(old_line) = self.data.get_mut(line + self.start) {
            *old_line = new_line;
            self.modified = true;
        }
    }

    pub fn insert_line(&mut self, row: usize, line: Vec<char>) {
        if row < self.data.len() {
            self.data.insert(row + self.start, line);
        } else {
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

    pub fn delete_char(&mut self, line: usize, col: usize) {
        if let Some(line) = self.data.get_mut(line + self.start) {
            if col < line.len() {
                line.remove(col);
                self.modified = true;
            }
        }
    }

    pub fn insert_char(&mut self, row: usize, col: usize, c: char) {
        if let Some(line) = self.data.get_mut(row + self.start) {
            if col > line.len() {
                line.push(c);
            } else {
                self.modified = true;
            }
            line.insert(col, c);
        }
    }

    pub fn delete_line(&mut self, line: usize) {
        if line < self.data.len() {
            self.data.remove(line + self.start);
            self.modified = true;
        }
    }
}
