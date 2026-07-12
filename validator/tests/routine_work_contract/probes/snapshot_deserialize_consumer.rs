extern crate routine_surface;

use routine_surface::routine_work::DirtySnapshot;

fn deserialize(bytes: &[u8]) -> DirtySnapshot {
    serde_json::from_slice(bytes).unwrap()
}

fn main() {
    let _ = deserialize;
}
