use super::create_exclusive_at;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::symlink;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn launch_sink_stays_with_held_directory_after_path_substitution() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("launch-sink-race-{suffix}"));
    let named = root.join("launch");
    let held_path = root.join("launch-held");
    let outside = root.join("outside");
    fs::create_dir_all(&named).expect("launch directory");
    fs::create_dir_all(&outside).expect("outside directory");
    let directory = File::open(&named).expect("hold launch directory");

    fs::rename(&named, &held_path).expect("move held directory");
    symlink(&outside, &named).expect("substitute outside symlink");
    let mut marker =
        create_exclusive_at(&directory, "authority", 0o400).expect("descriptor-relative create");
    marker
        .write_all(b"bound\n")
        .expect("descriptor-relative write");
    marker.sync_all().expect("sync marker");

    assert_eq!(
        fs::read(held_path.join("authority")).expect("held marker"),
        b"bound\n"
    );
    assert!(!outside.join("authority").exists());
    fs::remove_file(&named).expect("remove symlink");
    fs::remove_dir_all(root).expect("cleanup");
}
