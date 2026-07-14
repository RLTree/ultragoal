use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SCRATCH: AtomicU64 = AtomicU64::new(0);

pub(crate) struct OwnedScratch {
    path: PathBuf,
}

impl OwnedScratch {
    pub(crate) fn claim(label: &str) -> Self {
        let root = configured_root("CODEX_WORKTREE_SCRATCH");
        let _ = configured_root("CODEX_WORKTREE_TMP");
        for _ in 0..64 {
            let nonce = NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!("{label}-{}-{nonce}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
                    return Self { path };
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("issuer scratch claim failed: {error}"),
            }
        }
        panic!("issuer scratch claim collisions exhausted")
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for OwnedScratch {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let _ = fs::write(
                self.path.join("RETAINED-FAILURE.txt"),
                b"issuer visibility failure retained under configured scratch\n",
            );
        } else {
            fs::remove_dir_all(&self.path).expect("owned issuer scratch cleanup");
        }
    }
}

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
];

pub(crate) fn prepare(scratch: &Path, probes: &Path) {
    let validator = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut manifest = format!(
        "[workspace]\n\n[package]\nname = \"n06-issuer-visibility\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\nname = \"public_surface\"\npath = \"public_surface.rs\"\n\n[dependencies]\nultragoal = {{ path = {:?} }}\n",
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
    let docs = scratch.join("target/doc/public_surface");
    if docs.exists() {
        fs::remove_dir_all(&docs).unwrap();
    }
    let output = cargo(scratch)
        .args(["rustdoc", "--offline", "--quiet", "--lib"])
        .output()
        .unwrap();
    fs::write(scratch.join("rustdoc.stderr"), &output.stderr).unwrap();
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
        .env("CARGO_TARGET_DIR", scratch.join("target"))
        .env("TMPDIR", configured_root("CODEX_WORKTREE_TMP"))
        .env("TMP", configured_root("CODEX_WORKTREE_TMP"))
        .env("TEMP", configured_root("CODEX_WORKTREE_TMP"));
    command
}

fn configured_root(name: &str) -> PathBuf {
    let value = std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required"));
    let path = fs::canonicalize(PathBuf::from(value))
        .unwrap_or_else(|error| panic!("{name} is unavailable: {error}"));
    assert!(path.is_absolute() && path.is_dir(), "invalid {name}");
    path
}
