use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) fn compile_surface(scratch: &Path, dependencies: &Path) -> PathBuf {
    let routine = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routine_work/mod.rs");
    let source = scratch.join("routine_surface.rs");
    fs::write(
        &source,
        format!(
            "pub mod context {{ pub use ultragoal::context::*; }}\npub mod capture {{ pub use ultragoal::capture::*; }}\n#[path = {routine:?}]\npub mod routine_work;\n"
        ),
    )
    .unwrap();
    let surface = scratch.join("libroutine_surface.rlib");
    let mut command = Command::new(rustc());
    command.args([
        OsStr::new("--edition"),
        OsStr::new("2024"),
        OsStr::new("--crate-name"),
        OsStr::new("routine_surface"),
        OsStr::new("--crate-type"),
        OsStr::new("rlib"),
    ]);
    command.arg(&source).arg("-o").arg(&surface);
    dependency_path(&mut command, dependencies);
    for name in [
        "ultragoal",
        "serde",
        "serde_json",
        "sha2",
        "libc",
        "getrandom",
        "hmac",
    ] {
        external(&mut command, name, &find_rlib(dependencies, name));
    }
    external(&mut command, "syn", &find_syn_full_rlib(dependencies));
    let output = command.output().unwrap();
    fs::write(scratch.join("surface.stderr"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "public-surface wrapper failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    surface
}

pub(crate) fn compile_consumer(
    source: &Path,
    crate_name: &str,
    scratch: &Path,
    dependencies: &Path,
    surface: &Path,
) -> Output {
    let mut command = Command::new(rustc());
    command
        .arg("--edition")
        .arg("2024")
        .arg("--crate-name")
        .arg(crate_name)
        .arg("--emit")
        .arg("metadata")
        .arg(source)
        .arg("--out-dir")
        .arg(scratch);
    dependency_path(&mut command, dependencies);
    dependency_path(&mut command, scratch);
    external(&mut command, "routine_surface", surface);
    external(
        &mut command,
        "serde_json",
        &find_rlib(dependencies, "serde_json"),
    );
    command.output().unwrap()
}

pub(crate) fn compile_public_consumer(
    source: &Path,
    crate_name: &str,
    scratch: &Path,
    dependencies: &Path,
) -> Output {
    let mut command = Command::new(rustc());
    command
        .arg("--edition")
        .arg("2024")
        .arg("--crate-name")
        .arg(crate_name)
        .arg("--emit")
        .arg("metadata")
        .arg(source)
        .arg("--out-dir")
        .arg(scratch);
    dependency_path(&mut command, dependencies);
    external(
        &mut command,
        "ultragoal",
        &find_rlib(dependencies, "ultragoal"),
    );
    command.output().unwrap()
}

pub(crate) fn document_public_surface(scratch: &Path, dependencies: &Path) -> PathBuf {
    let source = scratch.join("public_surface.rs");
    if !source.exists() {
        fs::write(
            &source,
            "#[doc(inline)]\npub use ultragoal::routine_work::*;\n",
        )
        .unwrap();
    }
    let docs = scratch.join("public-docs");
    if docs.exists() {
        fs::remove_dir_all(&docs).unwrap();
    }
    let mut command = Command::new(rustdoc());
    command
        .arg("--edition")
        .arg("2024")
        .arg("--crate-name")
        .arg("public_surface")
        .arg("--crate-type")
        .arg("lib")
        .arg(&source)
        .arg("-o")
        .arg(&docs);
    dependency_path(&mut command, dependencies);
    external(
        &mut command,
        "ultragoal",
        &find_rlib(dependencies, "ultragoal"),
    );
    let output = command.output().unwrap();
    fs::write(scratch.join("public-rustdoc.stderr"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    docs
}

pub(crate) fn document_surface(scratch: &Path, dependencies: &Path) -> PathBuf {
    let source = scratch.join("routine_surface.rs");
    let docs = scratch.join("docs");
    if docs.exists() {
        fs::remove_dir_all(&docs).unwrap();
    }
    let mut command = Command::new(rustdoc());
    command
        .arg("--edition")
        .arg("2024")
        .arg("--crate-name")
        .arg("routine_surface")
        .arg("--crate-type")
        .arg("lib")
        .arg(&source)
        .arg("-o")
        .arg(&docs);
    dependency_path(&mut command, dependencies);
    for name in [
        "ultragoal",
        "serde",
        "serde_json",
        "sha2",
        "libc",
        "getrandom",
        "hmac",
    ] {
        external(&mut command, name, &find_rlib(dependencies, name));
    }
    external(&mut command, "syn", &find_syn_full_rlib(dependencies));
    let output = command.output().unwrap();
    fs::write(scratch.join("rustdoc.stderr"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "public API documentation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    docs
}

fn dependency_path(command: &mut Command, path: &Path) {
    command
        .arg("-L")
        .arg(format!("dependency={}", path.display()));
}

fn external(command: &mut Command, name: &str, path: &Path) {
    command
        .arg("--extern")
        .arg(format!("{name}={}", path.display()));
}

fn find_rlib(dependencies: &Path, name: &str) -> PathBuf {
    let prefix = format!("lib{name}-");
    fs::read_dir(dependencies)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension() == Some(OsStr::new("rlib"))
                && path
                    .file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|file| file.starts_with(&prefix))
        })
        .max_by_key(|path| fs::metadata(path).and_then(|value| value.modified()).ok())
        .unwrap_or_else(|| panic!("missing {name} rlib in {}", dependencies.display()))
}

fn find_syn_full_rlib(dependencies: &Path) -> PathBuf {
    let fingerprints = dependencies.parent().unwrap().join(".fingerprint");
    fs::read_dir(dependencies)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension() == Some(OsStr::new("rlib"))
                && path
                    .file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(|name| name.starts_with("libsyn-"))
        })
        .find(|path| {
            let hash = path
                .file_stem()
                .and_then(OsStr::to_str)
                .and_then(|stem| stem.strip_prefix("libsyn-"))
                .unwrap();
            fs::read_to_string(fingerprints.join(format!("syn-{hash}/lib-syn.json")))
                .ok()
                .and_then(|value| serde_json::from_str::<serde_json::Value>(&value).ok())
                .and_then(|value| value["features"].as_str().map(str::to_owned))
                .is_some_and(|features| features.contains("\"full\""))
        })
        .expect("full syn rlib")
}

fn rustc() -> OsString {
    std::env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"))
}

fn rustdoc() -> OsString {
    std::env::var_os("RUSTDOC").unwrap_or_else(|| OsString::from("rustdoc"))
}
