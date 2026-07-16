use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

const BINS: &[(&str, &str)] = &[
    (
        "routine_public_api_control",
        "routine_public_api_control.rs",
    ),
    (
        "production_issuer_consumer",
        "production_issuer_consumer.rs",
    ),
    ("production_grant_consumer", "production_grant_consumer.rs"),
    (
        "production_grant_entrypoint_consumer",
        "production_grant_entrypoint_consumer.rs",
    ),
    (
        "production_private_module_consumer",
        "production_private_module_consumer.rs",
    ),
    (
        "production_private_grant_consumer",
        "production_private_grant_consumer.rs",
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
    let validator = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut manifest = format!(
        "[workspace]\n\n[package]\nname = \"routine-issuer-visibility-contract\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\nname = \"public_surface\"\npath = \"public_surface.rs\"\n\n[dependencies]\nserde_json = \"1\"\nultragoal = {{ path = {:?} }}\n",
        validator
    );
    for (name, file) in BINS {
        manifest.push_str(&format!(
            "\n[[bin]]\nname = \"{name}\"\npath = {:?}\n",
            probes.join(file)
        ));
    }
    fs::write(scratch.join("Cargo.toml"), manifest).unwrap();
    fs::write(
        scratch.join("public_surface.rs"),
        "#[doc(inline)]\npub use ultragoal::routine_work::*;\npub struct ApiSentinel;\n",
    )
    .unwrap();
}

pub(crate) fn check(scratch: &Path, bin: &str) -> Output {
    cargo(scratch)
        .args(["check", "--offline", "--quiet", "--bin", bin])
        .output()
        .unwrap()
}

pub(crate) fn document(scratch: &Path) -> PathBuf {
    let docs = compiler_target().join("doc/public_surface");
    let output = cargo(scratch)
        .args(["rustdoc", "--offline", "--quiet", "--lib"])
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

fn cargo(scratch: &Path) -> Command {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .current_dir(scratch)
        .env("CARGO_TARGET_DIR", compiler_target())
        .env("TMPDIR", configured_root("CODEX_WORKTREE_TMP"))
        .env("TMP", configured_root("CODEX_WORKTREE_TMP"))
        .env("TEMP", configured_root("CODEX_WORKTREE_TMP"));
    command
}

fn compiler_target() -> PathBuf {
    static TARGET: OnceLock<PathBuf> = OnceLock::new();
    TARGET
        .get_or_init(|| {
            configured_root("CARGO_TARGET_DIR")
                .join(format!("routine-issuer-api-cache-{}", std::process::id()))
        })
        .clone()
}

fn configured_root(name: &str) -> PathBuf {
    let value = std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required"));
    let path = fs::canonicalize(PathBuf::from(value))
        .unwrap_or_else(|error| panic!("{name} is unavailable: {error}"));
    assert!(path.is_absolute() && path.is_dir(), "invalid {name}");
    path
}
