use super::fixture::TestRoot;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;

#[test]
fn same_crate_sibling_cannot_compile_reserved_a_executed_b_substitution() {
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
    append_same_crate_mutant(&crate_root.join("src/orchestration/product/mod.rs"));

    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--lib", "--quiet"])
        .current_dir(&crate_root)
        .env("CARGO_TARGET_DIR", scratch.path().join("target"))
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "substitution mutant compiled");
    assert!(
        stderr.contains("no method named `complete_action`")
            && stderr.contains("ProductionRootAuthority"),
        "unexpected compile failure: {stderr}"
    );
}

fn append_same_crate_mutant(product_module: &Path) {
    let mut source = OpenOptions::new()
        .append(true)
        .open(product_module)
        .unwrap();
    source
        .write_all(
            br#"
mod reserved_a_executed_b_substitution_mutant {
    fn substitute(authority: &super::ProductionRootAuthority) {
        authority.complete_action((), |_root| {
            panic!("permit B and request B would execute here")
        });
    }
}
"#,
        )
        .unwrap();
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir(destination).unwrap();
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
