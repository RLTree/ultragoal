use std::cell::Cell;

pub(super) struct TransitionFlag(Cell<bool>);

impl TransitionFlag {
    pub(super) fn new() -> Self {
        Self(Cell::new(false))
    }

    pub(super) fn is_set(&self) -> bool {
        self.0.get()
    }

    pub(super) fn mark(&self) {
        self.0.set(true);
    }
}
