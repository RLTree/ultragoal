use super::support::{CANARY, resource_root, root, sibling, write_manifest};
use crate::package::inventory::{anchored::test_hooks, package_digest};
use std::fs;
use std::os::unix::net::UnixListener;

fn assert_rejected(error: &str) {
    assert!(error.contains("anchored package"), "{error}");
    assert!(!error.contains(CANARY), "canary escaped through {error}");
}

#[test]
fn ordinary_ancestor_symlink_substitution_cannot_escape_root() {
    let root = resource_root(
        "package-ordinary-ancestor-symlink",
        "docs/file.txt",
        b"trusted",
    );
    let outside = sibling(&root, "outside");
    fs::create_dir(&outside).expect("outside");
    fs::write(outside.join("file.txt"), CANARY).expect("canary");
    let docs = root.join("docs");
    let held = root.join("docs-held");
    let outside_for_hook = outside.clone();
    test_hooks::set_before_component("docs/file.txt", 0, move || {
        fs::rename(&docs, &held).expect("hold docs");
        std::os::unix::fs::symlink(&outside_for_hook, &docs).expect("substitute docs");
    });
    assert_rejected(&package_digest(&root).expect_err("ancestor symlink rejected"));
    fs::remove_dir_all(root).expect("cleanup root");
    fs::remove_dir_all(outside).expect("cleanup outside");
}

#[test]
fn ordinary_ancestor_real_directory_substitution_is_detected() {
    let root = resource_root(
        "package-ordinary-ancestor-real",
        "docs/file.txt",
        b"trusted",
    );
    let outside = sibling(&root, "replacement");
    fs::create_dir(&outside).expect("replacement");
    fs::write(outside.join("file.txt"), CANARY).expect("canary");
    let docs = root.join("docs");
    let held = root.join("docs-held");
    let replacement = root.join("docs");
    test_hooks::set_before_component("docs/file.txt", 0, move || {
        fs::rename(&docs, &held).expect("hold docs");
        fs::rename(&outside, &replacement).expect("substitute docs");
    });
    assert_rejected(&package_digest(&root).expect_err("real directory swap rejected"));
    fs::remove_dir_all(root).expect("cleanup root");
}

#[test]
fn manifest_and_root_substitution_fail_before_package_identity() {
    let manifest_root = resource_root("package-manifest-swap", "file.txt", b"trusted");
    let manifest = manifest_root.join("plugin-manifest-draft.json");
    let held = manifest_root.join("manifest-held.json");
    let evil = manifest_root.join("manifest-evil.json");
    fs::write(&evil, format!(r#"{{"resources":["{CANARY}"]}}"#)).expect("evil manifest");
    test_hooks::set_before_component("plugin-manifest-draft.json", 0, move || {
        fs::rename(&manifest, &held).expect("hold manifest");
        fs::rename(&evil, &manifest).expect("substitute manifest");
    });
    assert_rejected(&package_digest(&manifest_root).expect_err("manifest swap rejected"));
    fs::remove_dir_all(manifest_root).expect("cleanup manifest root");

    let root = resource_root("package-root-swap", "file.txt", b"trusted");
    let held = sibling(&root, "held");
    let replacement = sibling(&root, "replacement");
    fs::create_dir(&replacement).expect("replacement");
    fs::write(replacement.join("plugin-manifest-draft.json"), CANARY).expect("canary");
    let root_for_hook = root.clone();
    let replacement_for_hook = replacement.clone();
    test_hooks::set_before_root(move || {
        fs::rename(&root_for_hook, &held).expect("hold root");
        fs::rename(&replacement_for_hook, &root_for_hook).expect("replace root");
    });
    assert_rejected(&package_digest(&root).expect_err("root swap rejected"));
    fs::remove_dir_all(&root).expect("cleanup replacement root");
    fs::remove_dir_all(sibling(&root, "held")).expect("cleanup held root");
}

#[test]
fn root_replacement_during_session_is_rejected_at_finalization() {
    let root = resource_root("package-root-during-session", "file.txt", b"trusted");
    let held = sibling(&root, "held");
    let replacement = sibling(&root, "replacement");
    fs::create_dir(&replacement).expect("replacement");
    fs::write(replacement.join("plugin-manifest-draft.json"), CANARY).expect("canary");
    let root_for_hook = root.clone();
    let replacement_for_hook = replacement.clone();
    test_hooks::set_after_read("plugin-manifest-draft.json", move || {
        fs::rename(&root_for_hook, &held).expect("hold root");
        fs::rename(&replacement_for_hook, &root_for_hook).expect("replace root");
    });
    assert_rejected(&package_digest(&root).expect_err("mid-session root swap rejected"));
    fs::remove_dir_all(&root).expect("cleanup replacement root");
    fs::remove_dir_all(sibling(&root, "held")).expect("cleanup held root");
}

#[test]
fn unrelated_root_entry_churn_does_not_change_the_pinned_root_identity() {
    let root = resource_root("package-root-entry-churn", "file.txt", b"trusted");
    let unrelated = root.join("unlisted.tmp");
    test_hooks::set_after_read("plugin-manifest-draft.json", move || {
        fs::write(&unrelated, b"unlisted").expect("create unrelated root entry");
    });
    assert!(
        package_digest(&root)
            .expect("root identity remains pinned")
            .starts_with("sha256:")
    );
    fs::remove_dir_all(root).expect("cleanup root churn fixture");
}

#[test]
fn leaf_and_cross_resource_mutation_are_detected() {
    let leaf_root = resource_root("package-leaf-mutation", "docs/file.txt", b"trusted");
    let file = leaf_root.join("docs/file.txt");
    test_hooks::set_after_read("docs/file.txt", move || {
        fs::write(&file, CANARY).expect("mutate leaf");
    });
    assert_rejected(&package_digest(&leaf_root).expect_err("leaf mutation rejected"));
    fs::remove_dir_all(leaf_root).expect("cleanup leaf root");

    let root = root("package-cross-resource-mutation");
    fs::write(root.join("a.txt"), b"a").expect("a");
    fs::write(root.join("b.txt"), b"b").expect("b");
    write_manifest(&root, &["a.txt", "b.txt"]);
    let first = root.join("a.txt");
    test_hooks::set_before_component("b.txt", 0, move || {
        fs::write(&first, CANARY).expect("mutate first resource");
    });
    assert_rejected(&package_digest(&root).expect_err("mixed snapshot rejected"));
    fs::remove_dir_all(root).expect("cleanup cross-resource root");

    let root = resource_root("package-final-revalidation", "file.txt", b"trusted");
    let file = root.join("file.txt");
    test_hooks::set_before_finish(move || {
        fs::write(&file, CANARY).expect("mutate before finish");
    });
    assert_rejected(&package_digest(&root).expect_err("final mutation rejected"));
    fs::remove_dir_all(root).expect("cleanup final revalidation root");
}

#[test]
fn ordinary_hardlink_fifo_and_socket_are_rejected_without_opening() {
    let hard = resource_root("package-ordinary-hardlink", "file.txt", b"trusted");
    fs::hard_link(hard.join("file.txt"), hard.join("second.txt")).expect("hard link");
    assert_rejected(&package_digest(&hard).expect_err("hardlink rejected"));
    fs::remove_dir_all(hard).expect("cleanup hardlink");

    let fifo = root("package-ordinary-fifo");
    let fifo_path = fifo.join("special");
    let name = std::ffi::CString::new(fifo_path.to_string_lossy().as_bytes()).expect("fifo name");
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    write_manifest(&fifo, &["special"]);
    assert_rejected(&package_digest(&fifo).expect_err("fifo rejected"));
    fs::remove_dir_all(fifo).expect("cleanup fifo");

    let socket = std::env::temp_dir().join(format!("n02-pkg-sock-{}", std::process::id()));
    fs::create_dir(&socket).expect("socket root");
    let listener = UnixListener::bind(socket.join("special")).expect("socket");
    write_manifest(&socket, &["special"]);
    assert_rejected(&package_digest(&socket).expect_err("socket rejected"));
    drop(listener);
    fs::remove_dir_all(socket).expect("cleanup socket");
}
