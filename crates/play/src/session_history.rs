#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SessionHistory<T> {
    undo: Vec<T>,
    redo: Vec<T>,
}

impl<T> SessionHistory<T> {
    pub(crate) fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub(crate) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub(crate) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub(crate) fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub(crate) fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub(crate) fn clear_redo(&mut self) {
        self.redo.clear();
    }

    pub(crate) fn push_undo(&mut self, entry: T) {
        self.undo.push(entry);
    }

    pub(crate) fn pop_undo(&mut self) -> Option<T> {
        self.undo.pop()
    }

    pub(crate) fn push_redo(&mut self, entry: T) {
        self.redo.push(entry);
    }

    pub(crate) fn pop_redo(&mut self) -> Option<T> {
        self.redo.pop()
    }

    pub(crate) fn truncate_undo(&mut self, len: usize) {
        self.undo.truncate(len);
    }
}
