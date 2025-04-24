use std::{error::Error, fs::{self, File}, io::{stdout, BufRead, BufReader, Write}, str::FromStr};

pub struct Buffer {
    pub path: String,
    data: Vec<Vec<char>>,
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

        Ok(Buffer::new(path.to_string(), buffer_data))
    }
}

impl Buffer {
    pub fn new(path: String, data: Vec<Vec<char>>) -> Self {
        Self { path, data }
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

    pub fn write(&self) -> Result<(), std::io::Error>{
        fs::write(&self.path, self.bytes())
    }

    pub fn print(&self) {
        for (i, line) in self.data.iter().enumerate() {
            for char in line {
                print!("{}", char);
            }
            
            if i != self.data.len() - 1 {
                println!();
            }
        }
        
        // Make sure all print!() output shows up before exiting
        stdout().flush().unwrap();
    }

    pub fn get_line(&self, line: usize) -> Option<&Vec<char>>{
        self.data.get(line)
    }

    pub fn get_char(&self, line: usize, col: usize) -> Option<&char> {
        self.data.get(line).and_then(|l| l.get(col))
    }

    pub fn delete_char(&mut self, line: usize, col: usize) {
        if let Some(line) = self.data.get_mut(line) {
            if col < line.len() {
                line.remove(col);
            }
        }
    }

    pub fn insert_char(&mut self, line: usize, col: usize, c: char) {
        if let Some(line) = self.data.get_mut(line) {
            if col > line.len() {
                line.push(c);
            } else {
                line.insert(col, c);
            }
        }
    }

    pub fn delete_line(&mut self, line: usize) {
        if line < self.data.len() {
            self.data.remove(line);
        }
    }
}