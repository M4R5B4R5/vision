use std::{fs, io::stdout, process::exit, str::FromStr};

use crossterm::{cursor::MoveTo, execute, terminal::{disable_raw_mode, Clear, ClearType}};

use crate::Buffer;

pub trait Run {
    fn run(&self, buffer: &Buffer) -> Result<(), RunError>;
}

pub enum Command {
    Quit(QuitCommand),
    Save(SaveCommand),
}

#[derive(Debug, Clone, Copy)]
pub enum CommandError {
    UnknownCommand,
}

#[derive(Debug, Clone, Copy)]
pub enum RunError {
    UnknownPath,
    QuitOnModified
}

impl FromStr for Command {
    type Err = CommandError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "q" => Ok(Self::Quit(QuitCommand { discard: false })),
            "q!" => Ok(Self::Quit(QuitCommand { discard: true })),
            "s" => Ok(Self::Save(SaveCommand)),
            _ => Err(CommandError::UnknownCommand),
        }
    }
}

impl Command {
    pub fn run(&self, buffer: &Buffer) -> Result<(), RunError> {
        match self {
            Self::Quit(quit) => quit.run(buffer),
            Self::Save(save) => save.run(buffer),
        }
    }
}

pub struct QuitCommand {
    discard: bool,
}

impl Run for QuitCommand {
    fn run(&self, buffer: &Buffer) -> Result<(), RunError> {
        if self.discard || !buffer.modified {
            execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0)).unwrap();
            print!("\x1b[3J");
            disable_raw_mode().unwrap();
            exit(0);
        } else {
            Err(RunError::QuitOnModified)
        } 
    }
}

pub struct SaveCommand;

impl Run for SaveCommand {
    fn run(&self, buffer: &Buffer) -> Result<(), RunError> {
        if buffer.path.is_none() {
            return Err(RunError::UnknownPath)
        }

        if buffer.modified {
            buffer.write().expect("Failed to write to disk while saving file");
        }

        Ok(())
    }
}
