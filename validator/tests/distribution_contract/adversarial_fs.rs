use std::fs::{self, OpenOptions};

use crate::distribution::{DistributionErrorId, Layer, verify};
use crate::distribution_fixture::{Fixture, identity_path, payload_path};

#[cfg(unix)]
#[test]
fn symlink_ancestor_hardlink_and_special_objects_are_rejected_without_opening() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;

    let leaf = Fixture::complete("leaf-symlink");
    let path = leaf.root.join(identity_path(Layer::Runtime));
    fs::remove_file(&path).unwrap();
    symlink("source-plugin-metadata.json", &path).unwrap();
    assert_error(
        verify(&leaf.root, &leaf.bytes()).unwrap_err(),
        DistributionErrorId::UnsafeObject,
    );

    let ancestor = Fixture::complete("ancestor-symlink");
    fs::rename(
        ancestor.root.join("identity"),
        ancestor.root.join("identity-real"),
    )
    .unwrap();
    symlink("identity-real", ancestor.root.join("identity")).unwrap();
    assert_error(
        verify(&ancestor.root, &ancestor.bytes()).unwrap_err(),
        DistributionErrorId::UnsafeObject,
    );

    let hardlink = Fixture::complete("hardlink");
    fs::hard_link(
        hardlink.root.join(identity_path(Layer::Discovery)),
        hardlink.root.join("identity/discovery-alias.json"),
    )
    .unwrap();
    assert_error(
        verify(&hardlink.root, &hardlink.bytes()).unwrap_err(),
        DistributionErrorId::UnsafeObject,
    );

    let special = Fixture::complete("special-fifo");
    let path = special.root.join(identity_path(Layer::Runtime));
    fs::remove_file(&path).unwrap();
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert_error(
        verify(&special.root, &special.bytes()).unwrap_err(),
        DistributionErrorId::UnsafeObject,
    );
}

#[test]
fn missing_oversized_and_invalid_paths_fail_closed_without_path_echo() {
    let missing = Fixture::complete("missing-file-canary");
    fs::remove_file(missing.root.join(identity_path(Layer::MarketplaceCatalog))).unwrap();
    assert_error(
        verify(&missing.root, &missing.bytes()).unwrap_err(),
        DistributionErrorId::ObjectUnavailable,
    );

    let oversized = Fixture::complete("oversized-payload");
    let payload = oversized.root.join(payload_path(Layer::CacheBytes));
    OpenOptions::new()
        .write(true)
        .open(payload)
        .unwrap()
        .set_len(4 * 1024 * 1024 + 1)
        .unwrap();
    assert_error(
        verify(&oversized.root, &oversized.bytes()).unwrap_err(),
        DistributionErrorId::ObjectTooLarge,
    );

    let mut escaped = Fixture::complete("escaped-path-canary");
    escaped.layer_row_mut(Layer::Runtime)["identity_path"] = serde_json::json!("../canary");
    assert_error(
        verify(&escaped.root, &escaped.bytes()).unwrap_err(),
        DistributionErrorId::InvalidPath,
    );
}

fn assert_error(error: crate::distribution::DistributionError, id: DistributionErrorId) {
    assert_eq!(error.id(), id);
    assert!(error.to_string().len() <= 64);
    assert!(!error.to_string().contains("canary"));
}
