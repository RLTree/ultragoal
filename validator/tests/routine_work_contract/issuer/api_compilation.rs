use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BINS: &[(&str, &str)] = &[
    (
        "routine_public_api_control",
        "routine_public_api_control.rs",
    ),
    (
        "production_issuer_consumer",
        "production/issuer_consumer.rs",
    ),
    ("production_grant_consumer", "production/grant_consumer.rs"),
    (
        "production_grant_entrypoint_consumer",
        "production/grant_entrypoint_consumer.rs",
    ),
    (
        "production_private_module_consumer",
        "production/private_module_consumer.rs",
    ),
    (
        "production_private_grant_consumer",
        "production/private_grant_consumer.rs",
    ),
    (
        "production_raw_custody_consumer",
        "production_raw_custody_consumer.rs",
    ),
    (
        "routine_snapshot_read_control",
        "read_only_snapshot_consumer.rs",
    ),
    (
        "routine_snapshot_constructor_attack",
        "snapshot_constructor_consumer.rs",
    ),
    (
        "routine_partial_snapshot_attack",
        "partial_snapshot_consumer.rs",
    ),
    (
        "routine_snapshot_deserialize_attack",
        "snapshot_deserialize_consumer.rs",
    ),
    (
        "routine_evidence_reconstruction_attack",
        "evidence_reconstruction_consumer.rs",
    ),
];

pub(crate) fn prepare(scratch: &Path, probes: &Path) {
    let _ = probes;
    fs::write(
        scratch.join("public_surface.rs"),
        "#[doc(inline)]\npub use ultragoal::routine_work::*;\npub struct ApiSentinel;\n",
    )
    .unwrap();
}

pub(crate) fn check(scratch: &Path, bin: &str) -> Output {
    compiler()
        .arg(probe_path(bin))
        .args([
            "--crate-name",
            bin,
            "--edition",
            "2024",
            "--emit",
            "metadata",
        ])
        .arg("--extern")
        .arg(format!(
            "ultragoal={}",
            library_artifact("ultragoal").display()
        ))
        .arg("--extern")
        .arg(format!(
            "serde_json={}",
            library_artifact("serde_json").display()
        ))
        .arg("-L")
        .arg(format!("dependency={}", dependency_directory().display()))
        .arg("--out-dir")
        .arg(scratch.join("probe-output"))
        .output()
        .unwrap()
}

pub(crate) fn document(scratch: &Path) -> PathBuf {
    let docs = scratch.join("docs/public_surface");
    let output = rustdoc()
        .arg(scratch.join("public_surface.rs"))
        .args(["--crate-name", "public_surface", "--edition", "2024"])
        .arg("--extern")
        .arg(format!(
            "ultragoal={}",
            library_artifact("ultragoal").display()
        ))
        .arg("-L")
        .arg(format!("dependency={}", dependency_directory().display()))
        .arg("--out-dir")
        .arg(scratch.join("docs"))
        .output()
        .unwrap();
    fs::write(
        scratch.join("rustdoc.stderr"),
        &output.stderr[..output.stderr.len().min(64 * 1024)],
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    docs
}

fn probe_path(bin: &str) -> PathBuf {
    let (_, file) = BINS
        .iter()
        .find(|(name, _)| *name == bin)
        .unwrap_or_else(|| panic!("unknown routine probe: {bin}"));
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/routine_work_contract/probes")
        .join(file)
}

fn compiler() -> Command {
    Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
}

fn rustdoc() -> Command {
    Command::new(std::env::var_os("RUSTDOC").unwrap_or_else(|| "rustdoc".into()))
}

fn dependency_directory() -> PathBuf {
    configured_root("CARGO_TARGET_DIR").join("debug/deps")
}

fn library_artifact(crate_name: &str) -> PathBuf {
    let prefix = format!("lib{crate_name}-");
    let mut artifacts = fs::read_dir(dependency_directory())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name().is_some_and(|name| {
                let name = name.to_string_lossy();
                name.starts_with(&prefix) && name.ends_with(".rlib")
            })
        })
        .collect::<Vec<_>>();
    artifacts.sort();
    artifacts
        .pop()
        .unwrap_or_else(|| panic!("compiled library artifact missing: {crate_name}"))
}

fn configured_root(name: &str) -> PathBuf {
    let value = std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required"));
    let path = fs::canonicalize(PathBuf::from(value))
        .unwrap_or_else(|error| panic!("{name} is unavailable: {error}"));
    assert!(path.is_absolute() && path.is_dir(), "invalid {name}");
    path
}
