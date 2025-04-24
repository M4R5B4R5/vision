use std::path::Path;
use clap::Parser;
use vision::{Buffer, Editor};

#[derive(Parser)]
struct Args {
    path_str: Option<String>,
}

fn run(args: Args) {
    let buffer = match args.path_str {
        Some(path_str) => {
            let path = Path::new(&path_str);
            match path.is_file() {
                true => path_str.parse::<Buffer>().expect("Could not open file. Path is invalid."),
                false => Buffer::new(Some(path_str)),
            }
        },
        None => Buffer::new(None),
    };

    let mut editor = Editor::new(buffer);

    editor.render();
    editor.cursor_home();
    editor.listen();
}

fn main() {
    run(Args::parse());
}
