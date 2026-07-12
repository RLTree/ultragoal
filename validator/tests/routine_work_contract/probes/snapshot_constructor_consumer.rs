extern crate routine_surface;

use routine_surface::routine_work::{DirtyChange, DirtySnapshot, RoutineBinding, RoutineError};

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
