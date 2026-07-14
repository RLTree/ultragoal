extern crate ultragoal;

use ultragoal::routine_work::{DirtyChange, DirtySnapshot, RoutineBinding};

fn inspect(snapshot: &DirtySnapshot) {
    let _: &str = snapshot.snapshot_id();
    let _: &RoutineBinding = snapshot.binding();
    let _: &str = snapshot.status_sha256();
    let _: &[DirtyChange] = snapshot.changes();
    let _: bool = snapshot.is_clean();
    let _: DirtySnapshot = snapshot.clone();
}

fn main() {
    let _ = inspect as fn(&DirtySnapshot);
}
