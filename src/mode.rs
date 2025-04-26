use std::{io::{stdout, Write}, thread, time::Duration};

use crossterm::{cursor::{position, MoveDown, MoveLeft, MoveRight, MoveTo, MoveToNextLine, MoveToPreviousLine, MoveUp}, event::{read, Event, KeyCode, KeyModifiers}, execute, style::Color, terminal::{enable_raw_mode, Clear, ClearType}};

use crate::{print_bg, print_fg, utils, Action, Command, Direction, Editor};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Command,
}

impl Mode {
    pub fn get(&self) -> Box<dyn ModeBehaviour> {
        match self {
            Self::Normal => Box::new(NormalMode),
            Self::Insert => Box::new(InsertMode),
            Self::Command => Box::new(CommandMode),
        }
    }
}

pub trait ModeBehaviour {
    fn listen(&mut self, editor: &mut Editor);
    fn print(&self);
}

pub struct NormalMode;
pub struct InsertMode;
pub struct CommandMode;

impl ModeBehaviour for NormalMode {
    fn print(&self) {
        print_fg!(Color::Green, "--NORMAL MODE--");
    }

    fn listen(&mut self, editor: &mut Editor) {
        enable_raw_mode().unwrap();
        while let Ok(event) = read() {
            match event {
                Event::Key(key_event) => {
                    let normal_pos = position().unwrap();
                    match key_event.code {
                        // Other mode listeners
                        KeyCode::Char(':') => CommandMode.listen(editor),
                        KeyCode::Char('i') => InsertMode.listen(editor),

                        // Cursor movement
                        KeyCode::Char('h') => {
                            editor.move_cursor(Direction::Left);
                        }
                        KeyCode::Char('k') => {
                            editor.move_cursor(Direction::Up);
                        }
                        KeyCode::Char('l') => {
                            editor.move_cursor(Direction::Right);
                        }
                        KeyCode::Char('j') => {
                            editor.move_cursor(Direction::Down);
                        }

                        // Shortcuts comming soon
                        KeyCode::Char('u') => {
                            editor.file.undo();
                            editor.render();
                        },

                        KeyCode::Char('r') => {
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                                editor.file.redo();
                                editor.render();
                            }
                        },
                        _ => {}
                    }

                    editor.set_mode(Mode::Normal);

                    // Reposition the cursor if it came out of command mode
                    if key_event.code == KeyCode::Char(':') {
                        execute!(stdout(), MoveTo(normal_pos.0, normal_pos.1)).unwrap();
                    }

                    enable_raw_mode().unwrap();
                }
                Event::Resize(col, row) => {
                    editor.render();
                }
                _ => {}
            }
        }
    }
}

impl ModeBehaviour for InsertMode {
    fn print(&self) {
        print_fg!(Color::Yellow, "--INSERT MODE--");
    }

    fn listen(&mut self, editor: &mut Editor) {
        editor.set_mode(Mode::Insert);

        while let Ok(event) = read() {
            if let Some(key_event) = event.as_key_event() {
                let (col, row) = position().unwrap();
                match key_event.code {
                    KeyCode::Esc => break,
                    KeyCode::Tab => self.process_tab(editor, col, row),
                    KeyCode::Enter => self.process_enter(editor, col, row),
                    KeyCode::Backspace => self.process_backspace(editor, col, row),
                    KeyCode::Char(c) => self.process_char(editor, col, row, c),
                    _ => {}
                }
                editor.render();
            }
        }
    }
}

impl InsertMode {
    fn process_tab(&mut self, editor: &mut Editor, col: u16, row: u16) {
        for i in 0..4 {
            if (position().unwrap().0) % 4 == 0 && i != 0 {
                return;
            }
            self.process_char(editor, col, row, ' ');
        }
    }

    fn process_enter(&mut self, editor: &mut Editor, col: u16, mut row: u16) {
        let line = editor.file.get_line(row as usize).expect("Failed to get line");

        let first_half = (&line[0..col as usize]).to_vec();
        let mut second_half = (&line[col as usize..line.len()]).to_vec();

        // Indentation from previous line
        let mut indentation = Vec::new();
        for char in &first_half {
            if *char == ' ' {
                indentation.push(' ');
            } else {
                break;
            }
        }
        
        // Replace the current line with everything left of the cursor
        editor.file.set_line(row as usize, first_half.clone(), Action::Do);
        execute!(stdout(), MoveToNextLine(1)).unwrap();
        row += 1;
        
        // // Insert another new line if user presses enter betweeen braces
        let left_char = first_half.last().copied();
        let right_char = second_half.first().copied();
        
        if let (Some(left), Some(right)) = (left_char, right_char) {
            if utils::braces(left, right) {
                let mut new_line = indentation.clone();
                new_line.extend(vec![' ', ' ', ' ', ' ']);
                editor.file.insert_line(row as usize, new_line, Action::Do);
                execute!(stdout(), MoveRight(4)).unwrap();
                row += 1;
            }
        }

        // Indent the second half
        second_half.splice(0..0, indentation.clone());

        // Insert second half
        editor.file.insert_line(row as usize, second_half, Action::Do);

        if indentation.len() > 0 {
            execute!(stdout(), MoveRight(indentation.len() as u16)).unwrap();
        }
    }

    fn process_backspace(&mut self, editor: &mut Editor, col: u16, row: u16) {
        // Don't do anything if user tries to delete the first column of the first row
        if row == 0 && col == 0 {
            return;
        }

        // Delete a character if the cursor is NOT at the start of the line
        let line = editor.file.get_line(row as usize).expect("Buffer has no starting line");
        if col != 0 {
            let left_char = line.get(col as usize - 1).copied();
            let right_char = line.get(col as usize).copied();

            if let (Some(left), Some(right)) = (left_char, right_char) {
                if utils::pair(left, right) {
                    editor.file.delete_char(row as usize, col as usize, Action::Do);
                }
            }

            editor.file.delete_char(row as usize, (col - 1) as usize, Action::Do);
            execute!(stdout(), MoveLeft(1)).unwrap();
            return;
        }

        // Otherwise, join the previous line with the next line (inverse operation of enter)
        let mut prev_line = editor
            .file
            .get_line(row as usize - 1)
            .expect("Failed to get prev_line")
            .clone();
        let prev_line_len = prev_line.len();
        prev_line.extend(line);
        editor.file.set_line(row as usize - 1, prev_line, Action::Do);

        execute!(stdout(), MoveUp(1)).unwrap();

        // Move the cursor to end of line if previous line isn't empty
        if prev_line_len > 0 {
            execute!(stdout(), MoveRight(prev_line_len as u16)).unwrap();
        }

        // Delete current line and move cursor up
        editor.file.delete_line(row as usize, Action::Do);
    }

    fn process_char(&mut self, editor: &mut Editor, mut col: u16, row: u16, c: char) {
        // If user types a closing literal that is preceeded by it's corresponding opening literal,
        // just move the cursor right
        if col != 0 {
            let line = editor.file.get_line(row as usize).expect("Buffer has no starting line");
            let right_char = line.get(col as usize).copied();
            
            if let Some(right) = right_char {
                if right == c && utils::openeable(c).is_some() {
                    execute!(stdout(), MoveRight(1)).unwrap();
                    return;
                }
            }
        }

        // Otherwise, insert whatever the user types
        editor.file.insert_char(row as usize, col as usize, c, Action::Do);
        col += 1;
        execute!(stdout(), MoveRight(1)).unwrap();

        // If inserted char is a literal that is part of a pair, insert it's corresponding partner also
        if let Some(closing) = utils::closeable(c) {
            editor.file.insert_char(row as usize, col as usize, closing, Action::Do);
            return;
        }
    }
}

impl ModeBehaviour for CommandMode {
    fn print(&self) {
        print_fg!(Color::Magenta, "--COMMAND MODE--")
    }

    fn listen(&mut self, editor: &mut Editor) {
        editor.set_mode(Mode::Command);
        editor.cursor_command();

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
                            if let Err(e) = command.run(&mut editor.file) {
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
}
