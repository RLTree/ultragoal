use super::fixture::TestRoot;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn same_crate_sibling_cannot_mint_authority_or_call_direct_executors() {
    let scratch = TestRoot::new("same-crate-transaction-mutant", 0o700);
    let crate_root = scratch.path().join("validator");
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    fs::create_dir(&crate_root).unwrap();
    copy_tree(
        &repository_root.join("validator/src"),
        &crate_root.join("src"),
    );
    copy_tree(
        &repository_root.join("validator/build_support"),
        &crate_root.join("build_support"),
    );
    copy_tree(
        &repository_root.join("templates"),
        &scratch.path().join("templates"),
    );
    copy_tree(
        &repository_root.join(".codex/agents"),
        &scratch.path().join(".codex/agents"),
    );
    copy_tree(
        &repository_root.join("docs/ultragoal-contract-2026-07-successor-v2"),
        &scratch
            .path()
            .join("docs/ultragoal-contract-2026-07-successor-v2"),
    );
    fs::create_dir_all(crate_root.join("tests")).unwrap();
    fs::copy(
        repository_root.join("validator/tests/public_api_witness.rs"),
        crate_root.join("tests/public_api_witness.rs"),
    )
    .unwrap();
    fs::create_dir_all(crate_root.join("examples")).unwrap();
    fs::copy(
        repository_root.join("validator/examples/hct_inventory.rs"),
        crate_root.join("examples/hct_inventory.rs"),
    )
    .unwrap();
    fs::copy(
        repository_root.join("validator/build.rs"),
        crate_root.join("build.rs"),
    )
    .unwrap();
    fs::copy(
        repository_root.join("plugin-manifest-draft.json"),
        scratch.path().join("plugin-manifest-draft.json"),
    )
    .unwrap();
    let manifest = fs::read_to_string(repository_root.join("validator/Cargo.toml"))
        .unwrap()
        .replace("[lints]\nworkspace = true\n\n", "")
        + "\n[workspace]\n";
    fs::write(crate_root.join("Cargo.toml"), manifest).unwrap();
    let module = crate_root.join("src/orchestration/product/authority/production/mod.rs");
    let original = fs::read_to_string(&module).unwrap();
    let cases = [
        (
            "root-fields",
            "mod mutant { use super::RootAuthority; fn forge(actor: crate::orchestration::Actor) -> RootAuthority { RootAuthority { root_actor: actor, key: [0; 32] } } }",
            "private",
        ),
        (
            "execution-token",
            "mod mutant { use super::ExecutionAuthority; fn probe() {} }",
            "no `ExecutionAuthority` in",
        ),
        (
            "execution-request",
            "mod mutant { use super::execution_transaction::ExecutionRequest; fn probe(_: Option<ExecutionRequest<'_>>) {} }",
            "private",
        ),
        (
            "validated-reservation",
            "mod mutant { use super::ValidatedExecution; fn probe() -> ValidatedExecution<'static> { ValidatedExecution { permit_id: String::new(), request: panic!() } } }",
            "private",
        ),
        (
            "reserved-execution",
            "mod mutant { use super::ReservedExecution; fn probe() -> ReservedExecution<'static> { ReservedExecution { permit_id: String::new(), request: panic!() } } }",
            "private",
        ),
        (
            "root-clone",
            "mod mutant { use super::RootAuthority; fn probe(root: RootAuthority) { let _ = root.clone(); } }",
            "no method named `clone`",
        ),
        (
            "direct-resume",
            "mod mutant { fn probe() { let _ = crate::orchestration::product::resume; } }",
            "expected value, found module",
        ),
        (
            "direct-recover",
            "mod mutant { fn probe() { let _ = crate::orchestration::product::recover; } }",
            "expected value, found module",
        ),
        (
            "direct-reconcile",
            "mod mutant { fn probe() { let _ = crate::orchestration::product::reconcile; } }",
            "expected value, found module",
        ),
        (
            "nested-direct-executor",
            "mod mutant { fn probe() { let _ = super::execution_transaction::resume::execute; } }",
            "private",
        ),
    ];
    for (label, mutant, expected) in cases {
        assert_mutant_rejected(&crate_root, &module, &original, label, mutant, expected);
    }
}

fn assert_mutant_rejected(
    crate_root: &Path,
    module: &Path,
    original: &str,
    label: &str,
    mutant: &str,
    expected: &str,
) {
    fs::write(module, format!("{original}\n{mutant}\n")).unwrap();
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--lib", "--quiet"])
        .current_dir(crate_root)
        .env("CARGO_TARGET_DIR", crate_root.join("target"))
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{label} mutant compiled");
    assert!(
        stderr.contains(expected),
        "{label} failed for the wrong reason: {stderr}"
    );
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
