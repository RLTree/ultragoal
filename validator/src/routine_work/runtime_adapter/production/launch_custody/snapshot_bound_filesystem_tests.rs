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

#[test]
fn unique_launch_leaves_share_the_held_root_without_disturbing_unrelated_entries() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("launch-leaves-{suffix}"));
    fs::create_dir(&root).expect("launch root");
    let directory = File::open(&root).expect("hold launch root");
    fs::write(root.join("unrelated"), b"keep\n").expect("unrelated entry");

    let mut first =
        create_exclusive_at(&directory, "launch-first-program", 0o500).expect("first direct leaf");
    first.write_all(b"first\n").expect("write first");
    let mut second = create_exclusive_at(&directory, "launch-second-program", 0o500)
        .expect("second direct leaf");
    second.write_all(b"second\n").expect("write second");
    assert!(create_exclusive_at(&directory, "launch-first-program", 0o500).is_err());
    assert_eq!(fs::read(root.join("unrelated")).unwrap(), b"keep\n");

    fs::remove_dir_all(root).expect("cleanup");
}
