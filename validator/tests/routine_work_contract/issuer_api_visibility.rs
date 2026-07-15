use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use super::issuer_api_compilation;
use super::issuer_hidden_surface::{file_has_hidden_public_api, tree_has_hidden_public_api};
use super::owned_compile_scratch::OwnedCompileScratch;

#[test]
fn sealed_issuer_and_grant_entrypoints_are_not_externally_callable() {
    assert_sealed_issuer_and_grant_entrypoints_are_not_externally_callable();
}

pub(crate) fn assert_sealed_issuer_and_grant_entrypoints_are_not_externally_callable() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-visibility");
    let scratch = owned.path();
    let probes = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/routine_work_contract/probes");
    issuer_api_compilation::prepare(&scratch, &probes);

    let control = issuer_api_compilation::check(&scratch, "routine_public_api_control");
    assert!(control.status.success(), "{}", diagnostic(&control));

    for (probe, code) in [
        ("production_issuer_consumer.rs", "E0432"),
        ("production_grant_consumer.rs", "E0432"),
        ("production_grant_entrypoint_consumer.rs", "E0432"),
        ("production_private_module_consumer.rs", "E0603"),
        ("production_private_grant_consumer.rs", "E0603"),
    ] {
        let output = issuer_api_compilation::check(&scratch, probe.trim_end_matches(".rs"));
        assert_private_failure(&output, code, probe);
    }

    let docs = issuer_api_compilation::document(&scratch);
    let inventory = public_inventory(&docs);
    let digest = format!("{:x}", Sha256::digest(&inventory));
    assert_eq!(
        digest,
        "b132f14ce3ffbc2871dc6fd05f55bec497d32d582ad487682193ffa2ed550d3e"
    );
    let routine_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routine_work");
    assert!(!tree_has_hidden_public_api(&routine_root));

    let mut source = OpenOptions::new()
        .append(true)
        .open(scratch.join("public_surface.rs"))
        .unwrap();
    source
        .write_all(b"\n#[doc(hidden)]\npub fn hidden_grant_entrypoint() {}\n")
        .unwrap();
    source.flush().unwrap();
    assert!(file_has_hidden_public_api(
        &fs::read_to_string(scratch.join("public_surface.rs")).unwrap()
    ));
    for mutant in [
        "pub struct Carrier { #[doc(hidden)] pub grant: fn() }",
        "pub enum Authority { #[doc(hidden)] Grant }",
        "pub struct Api; impl Api { #[cfg_attr(all(), doc(hidden))] pub fn issue() {} }",
        "grant_api!();",
        "unsafe extern \"C\" { #[doc(hidden)] pub fn issue(); }",
        "struct Api; #[doc(hidden)] impl Api { pub fn issue() {} }",
        "#[doc(hidden)] unsafe extern \"C\" { pub fn issue(); }",
        "thread_local! { #[doc(hidden)] pub static HIDDEN_GRANT: Cell<bool> = const { Cell::new(false) }; }",
    ] {
        assert!(file_has_hidden_public_api(mutant), "missed: {mutant}");
    }
    source
        .write_all(b"\nimpl ApiSentinel { pub fn grant(&self) {} pub const GRANT: () = (); }\n")
        .unwrap();
    source.flush().unwrap();
    let mutated = public_inventory(&issuer_api_compilation::document(&scratch));
    assert_ne!(Sha256::digest(&mutated), Sha256::digest(&inventory));
    owned.teardown_after_assertions();
}

#[test]
fn concurrent_issuer_controls_use_disjoint_authorized_scratch() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-concurrency");
    let scratch = owned.path().join("scratch");
    let tmp = owned.path().join("tmp");
    fs::create_dir(&scratch).unwrap();
    fs::create_dir(&tmp).unwrap();
    let sentinel = scratch.join("parent-owned-sentinel");
    fs::write(&sentinel, b"must survive child cleanup\n").unwrap();
    let executable = std::env::current_exe().unwrap();
    let mut children = (0..2)
        .map(|_| {
            Command::new(&executable)
                .args([
                    "issuer_api_visibility::sealed_issuer_and_grant_entrypoints_are_not_externally_callable",
                    "--exact",
                ])
                .env("CODEX_WORKTREE_SCRATCH", &scratch)
                .env("CODEX_WORKTREE_TMP", &tmp)
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }
    assert_eq!(
        fs::read(&sentinel).unwrap(),
        b"must survive child cleanup\n"
    );
    fs::remove_file(sentinel).unwrap();
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&tmp).unwrap().count(), 0);
    owned.teardown_after_assertions();
}

fn public_inventory(docs: &Path) -> Vec<u8> {
    fn visit(root: &Path, current: &Path, rows: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries = fs::read_dir(current)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                visit(root, &path, rows);
            } else {
                rows.push((
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut rows = Vec::new();
    visit(docs, docs, &mut rows);
    let mut bytes = Vec::new();
    for (path, data) in rows {
        bytes.extend(path.as_os_str().as_encoded_bytes());
        bytes.push(0);
        bytes.extend(data);
    }
    bytes
}

fn assert_private_failure(output: &Output, code: &str, probe: &str) {
    let text = diagnostic(output);
    assert!(!output.status.success(), "{probe} became callable");
    assert!(text.contains(code), "unexpected {probe} failure: {text}");
    for incidental in ["can't find crate", "file not found", "couldn't read"] {
        assert!(!text.contains(incidental), "incidental {probe}: {text}");
    }
}

fn diagnostic(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
