extern crate ultragoal;

use ultragoal::routine_work::{DirtyChange, DirtySnapshot, RoutineBinding, RoutineError};

fn construct(
    binding: RoutineBinding,
    status_sha256: String,
    changes: Vec<DirtyChange>,
) -> Result<DirtySnapshot, RoutineError> {
    DirtySnapshot::new(binding, status_sha256, changes)
}

fn main() {
    let _ = construct;
}
