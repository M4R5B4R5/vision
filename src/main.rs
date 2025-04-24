use std::{error::Error, fs::File, io::{stdout, BufRead, BufReader}};

use clap::Parser;

use crossterm::{cursor::{MoveTo, SetCursorStyle}, event::{read, Event, KeyCode}, terminal::*, *};
use vision::{Buffer, Editor};

#[derive(Parser)]
struct Args {
    path: Option<String>,
}

fn open_existing(path: String) {
    let mut buffer = path.parse::<Buffer>().expect("Could not open file. Path is invalid.");
    let mut editor = Editor::new(buffer);

    editor.render();
    editor.listen();
}

fn create_new() {
    todo!()
}

fn main() {
    match Args::parse().path {
        Some(path) => open_existing(path),
        None => create_new(),
    }
}
