use super::{ProductFitnessError, read_stable};
use std::fs;
use std::os::unix::fs::symlink;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn evidence_ancestor_symlink_is_never_followed() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("fitness-evidence-{suffix}"));
    let outside = root.with_extension("outside");
    fs::create_dir_all(&root).expect("root");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(outside.join("evidence.json"), b"outside").expect("outside evidence");
    symlink(&outside, root.join("proof")).expect("ancestor symlink");

    assert_eq!(
        read_stable(&root, "proof/evidence.json").expect_err("ancestor symlink must fail closed"),
        ProductFitnessError::EvidencePathInvalid
    );

    fs::remove_dir_all(&root).expect("cleanup root");
    fs::remove_dir_all(outside).expect("cleanup outside");
}
