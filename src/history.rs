#[derive(Clone, Copy)]
pub enum Action {
    Do,
    Undo,
    Redo,
}

pub trait Undo {
    fn undo(&mut self); 
}

pub trait Redo {
    fn redo(&mut self);
}

pub struct History<T> {
    edits: Vec<T>,
    undos: Vec<T>
}

impl<T: Clone> History<T> {
    pub fn new() -> Self {
        Self { edits: vec![], undos: vec![]}
    }

    pub fn update(&mut self, event: T, action: Action) {
        match action {
            Action::Do => self.edits.push(event),
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

    pub fn last_from(&self, action: Action) -> Option<T> {
        match action {
            Action::Do | Action::Redo => self.undos.last().cloned(),
            Action::Undo => self.edits.last().cloned(),
        }
    }
}