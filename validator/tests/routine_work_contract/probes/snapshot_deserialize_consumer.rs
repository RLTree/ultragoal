extern crate ultragoal;

use ultragoal::routine_work::DirtySnapshot;

fn deserialize(bytes: &[u8]) -> DirtySnapshot {
    serde_json::from_slice(bytes).unwrap()
}

fn main() {
    let _ = deserialize;
}
