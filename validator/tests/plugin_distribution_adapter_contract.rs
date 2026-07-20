//! External boundary target for the retired plugin mutation sibling.
//!
//! The former adapter contract remains compiled in crate-private lifecycle
//! custody tests. This external target intentionally has no import path to the
//! retired surface; the public lifecycle module's compile-fail examples are
//! the source-level negative control.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn retired_mutation_symbols_are_external_compile_negative() {
    let target = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap());
    let deps = target.join("debug/deps");
    let library = fs::read_dir(&deps)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name().is_some_and(|name| {
                let name = name.to_string_lossy();
                name.starts_with("libultragoal-") && name.ends_with(".rlib")
            })
        })
        .expect("ultragoal rlib must exist for the external consumer control");
    let scratch = std::env::temp_dir().join(format!("hul-retired-adapter-{}", std::process::id()));
    fs::create_dir_all(&scratch).unwrap();
    let source = scratch.join("consumer.rs");
    fs::write(
        &source,
        "use ultragoal::plugin_product::lifecycle::{apply, LifecycleEffectAdapter};\nuse ultragoal::plugin_product::distribution_adapter::DistributionLifecycleOperation;\nfn main() {}\n",
    )
    .unwrap();
    let output = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args([
            "--edition",
            "2024",
            "--crate-name",
            "retired_adapter_consumer",
        ])
        .arg(&source)
        .arg("--extern")
        .arg(format!("ultragoal={}", library.display()))
        .arg("-L")
        .arg(format!("dependency={}", deps.display()))
        .arg("--emit=metadata")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "retired mutation surface compiled"
    );
    let _ = fs::remove_dir_all(scratch);
}
