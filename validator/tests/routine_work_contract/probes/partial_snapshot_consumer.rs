extern crate ultragoal;

use ultragoal::routine_work::{DirtyChange, DirtySnapshot};

fn clone_then_subset(snapshot: DirtySnapshot, changes: Vec<DirtyChange>) -> DirtySnapshot {
    DirtySnapshot {
        changes,
        ..snapshot
    }
}

fn main() {
    let _ = clone_then_subset;
}
