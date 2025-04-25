use std::{fs, io::stdout, process::exit, str::FromStr};

use crossterm::{cursor::MoveTo, execute, terminal::{disable_raw_mode, Clear, ClearType}};

use crate::Buffer;

pub trait Run {
    fn run(&self, buffer: &mut Buffer) -> Result<(), RunError>;
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

pub enum Command {
    Quit(QuitCommand),
    Save(SaveCommand),
    SaveQuit(SaveQuitCommand)
}

pub struct QuitCommand {
    discard: bool,
}
pub struct SaveCommand;

pub struct SaveQuitCommand;

impl FromStr for Command {
    type Err = CommandError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "q" => Ok(Self::Quit(QuitCommand { discard: false })),
            "q!" => Ok(Self::Quit(QuitCommand { discard: true })),
            "w" => Ok(Self::Save(SaveCommand)),
            "wq" => Ok(Self::SaveQuit(SaveQuitCommand)),
            _ => Err(CommandError::UnknownCommand),
        }
    }
}

impl Command {
    pub fn run(&self, buffer: &mut Buffer) -> Result<(), RunError> {
        match self {
            Self::Quit(quit) => quit.run(buffer),
            Self::Save(save) => save.run(buffer),
            Self::SaveQuit(save_quit) => save_quit.run(buffer),
        }
    }
}


impl Run for QuitCommand {
    fn run(&self, buffer: &mut Buffer) -> Result<(), RunError> {
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

impl Run for SaveCommand {
    fn run(&self, buffer: &mut Buffer) -> Result<(), RunError> {
        if buffer.path.is_none() {
            return Err(RunError::UnknownPath)
        }

        if buffer.modified {
            buffer.write().expect("Failed to write to disk while saving file");
            buffer.modified = false;
        }

        Ok(())
    }
}

impl Run for SaveQuitCommand {
    fn run(&self, buffer: &mut Buffer) -> Result<(), RunError> {
        SaveCommand.run(buffer)?;
        Ok(QuitCommand {discard: false}.run(buffer)?)
    }
}