use std::io::stdout;
use crossterm::{cursor::{position, MoveTo}, execute, style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor}, terminal::{size, Clear, ClearType}};

#[macro_export]
macro_rules! print_bg {
    ($color:expr, $($arg:tt)*) => {
        ::crossterm::execute!(
            ::std::io::stdout(),
            ::crossterm::style::SetBackgroundColor($color),
            ::crossterm::style::Print(format!($($arg)*)),
            ::crossterm::style::ResetColor
        ).unwrap();
    };
}

#[macro_export]
macro_rules! print_fg {
    ($color:expr, $($arg:tt)*) => {
        ::crossterm::execute!(
            ::std::io::stdout(),
            ::crossterm::style::SetForegroundColor($color),
            ::crossterm::style::Print(format!($($arg)*)),
            ::crossterm::style::ResetColor
        ).unwrap()
    };
}

pub fn clear_line() {
    execute!(
        stdout(),
        Clear(ClearType::CurrentLine),
        MoveTo(0, position().unwrap().1)
    ).unwrap();
}

pub fn window_size() -> u16 {
    size().unwrap().1 - 2
}