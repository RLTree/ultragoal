#[path = "../src/orchestration/mod.rs"]
mod orchestration;
#[path = "../src/orchestration/product/runtime_adapter/mod.rs"]
mod runtime_adapter;

use orchestration::product::command::{
    CommandProjection, InterruptedRecoveryRequest, OrchestrationStateRequest, RootActionRequest,
};
use orchestration::product::{
    ProductContext, ProductError, ProductWorkspace, ReconcileOutcome, ReconcileRequest,
    RootAuthority, RootPermit,
};
use runtime_adapter::{
    CurrentRuntimeView, InterruptedRuntimeView, OrchestrationRuntimeAdapter, RuntimeActionOutcome,
    RuntimeActionRequest, RuntimeActionSource,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn root_wiring_surface_is_typed_without_exposing_authority_construction() {
    fn inspect_current<'a>(
        adapter: &OrchestrationRuntimeAdapter<'a>,
        request: &OrchestrationStateRequest,
    ) -> Result<CurrentRuntimeView, ProductError> {
        adapter.inspect_current(request)
    }
    fn inspect_interrupted<'a>(
        adapter: &OrchestrationRuntimeAdapter<'a>,
        request: &InterruptedRecoveryRequest,
    ) -> Result<InterruptedRuntimeView, ProductError> {
        adapter.inspect_interrupted(request)
    }
    fn project<'a>(
        adapter: &OrchestrationRuntimeAdapter<'a>,
        view: &CurrentRuntimeView,
        projection: &CommandProjection,
    ) -> Result<Option<Vec<u8>>, ProductError> {
        adapter.project_current(view, projection)
    }
    fn execute_action<'a>(
        adapter: &OrchestrationRuntimeAdapter<'a>,
        source: RuntimeActionSource<'_>,
        action: &RootActionRequest,
        authority: &RootAuthority,
        permit: &RootPermit,
        request: &RuntimeActionRequest,
    ) -> Result<RuntimeActionOutcome, ProductError> {
        adapter.execute_action(source, action, authority, permit, request)
    }
    fn execute_reconcile<'a>(
        adapter: &OrchestrationRuntimeAdapter<'a>,
        current: &CurrentRuntimeView,
        action: &RootActionRequest,
        authority: &RootAuthority,
        permit: &RootPermit,
        request: &ReconcileRequest,
    ) -> Result<ReconcileOutcome, ProductError> {
        adapter.execute_reconcile(current, action, authority, permit, request)
    }

    let _ = inspect_current;
    let _ = inspect_interrupted;
    let _ = project;
    let _ = execute_action;
    let _ = execute_reconcile;
    assert!(
        std::any::type_name::<OrchestrationRuntimeAdapter<'static>>()
            .ends_with("runtime_adapter::view::OrchestrationRuntimeAdapter<'_>")
    );
}

#[test]
fn exact_root_export_compiles_from_an_external_consumer_crate() {
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    let scratch = Scratch(PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "orchestration-runtime-adapter-public-api-{}",
        std::process::id()
    )));
    if scratch.0.exists() {
        fs::remove_dir_all(&scratch.0).unwrap();
    }
    fs::create_dir_all(scratch.0.join("ultragoal")).unwrap();
    fs::create_dir_all(scratch.0.join("consumer/src")).unwrap();

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository = manifest.parent().unwrap();
    copy_tree(&manifest.join("src"), &scratch.0.join("ultragoal/src"));
    copy_tree(
        &repository.join(".codex/agents"),
        &scratch.0.join(".codex/agents"),
    );
    copy_tree(
        &repository.join("docs/ultragoal-contract-2026-07-successor-v2"),
        &scratch
            .0
            .join("docs/ultragoal-contract-2026-07-successor-v2"),
    );
    fs::create_dir_all(scratch.0.join("ultragoal/tests")).unwrap();
    fs::create_dir_all(scratch.0.join("ultragoal/examples")).unwrap();
    fs::copy(
        manifest.join("tests/public_api_witness.rs"),
        scratch.0.join("ultragoal/tests/public_api_witness.rs"),
    )
    .unwrap();
    fs::copy(
        manifest.join("examples/hct_inventory.rs"),
        scratch.0.join("ultragoal/examples/hct_inventory.rs"),
    )
    .unwrap();
    fs::copy(
        manifest.join("Cargo.toml"),
        scratch.0.join("ultragoal/Cargo.toml"),
    )
    .unwrap();

    let product_mod = scratch.0.join("ultragoal/src/orchestration/product/mod.rs");
    let source = fs::read_to_string(&product_mod).unwrap();
    let existing = source.matches("pub mod runtime_adapter;").count();
    assert!(existing <= 1);
    let wired = if existing == 0 {
        source.replacen(
            "pub mod command;\n",
            "pub mod command;\npub mod runtime_adapter;\n",
            1,
        )
    } else {
        source
    };
    assert_eq!(wired.matches("pub mod runtime_adapter;").count(), 1);
    fs::write(&product_mod, wired).unwrap();

    fs::write(
        scratch.0.join("Cargo.toml"),
        "[workspace]\nmembers = [\"ultragoal\", \"consumer\"]\nresolver = \"3\"\n",
    )
    .unwrap();
    fs::write(
        scratch.0.join("consumer/Cargo.toml"),
        "[package]\nname = \"runtime-adapter-consumer\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[dependencies]\nultragoal = { path = \"../ultragoal\" }\n",
    )
    .unwrap();
    fs::write(
        scratch.0.join("consumer/src/main.rs"),
        r#"use ultragoal::orchestration::product::command::{
    CommandProjection, InterruptedRecoveryRequest, OrchestrationStateRequest, RootActionRequest,
};
use ultragoal::orchestration::product::runtime_adapter::{
    CurrentRuntimeView, InterruptedRuntimeView, OrchestrationRuntimeAdapter, RuntimeActionOutcome,
    RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    ProductError, ReconcileOutcome, ReconcileRequest, RootAuthority, RootPermit,
};

fn consume<'a>(
    adapter: &OrchestrationRuntimeAdapter<'a>,
    current_request: &OrchestrationStateRequest,
    interrupted_request: &InterruptedRecoveryRequest,
    current: &CurrentRuntimeView,
    interrupted: &InterruptedRuntimeView,
    action: &RootActionRequest,
    authority: &RootAuthority,
    permit: &RootPermit,
    request: &RuntimeActionRequest,
) -> Result<(Option<Vec<u8>>, RuntimeActionOutcome), ProductError> {
    let _ = adapter.inspect_current(current_request)?;
    let _ = adapter.inspect_interrupted(interrupted_request)?;
    let projected = adapter.project_current(current, &CommandProjection::Inspect)?;
    let outcome = adapter.execute_action(
        RuntimeActionSource::Interrupted(interrupted),
        action,
        authority,
        permit,
        request,
    )?;
    Ok((projected, outcome))
}

fn consume_reconcile<'a>(
    adapter: &OrchestrationRuntimeAdapter<'a>,
    current: &CurrentRuntimeView,
    action: &RootActionRequest,
    authority: &RootAuthority,
    permit: &RootPermit,
    request: &ReconcileRequest,
) -> Result<ReconcileOutcome, ProductError> {
    adapter.execute_reconcile(current, action, authority, permit, request)
}

fn main() {
    let _ = consume;
    let _ = consume_reconcile;
}
"#,
    )
    .unwrap();

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let target = scratch.0.join("target");
    let generated = Command::new(&cargo)
        .args(["generate-lockfile", "--offline"])
        .current_dir(&scratch.0)
        .env("CARGO_TARGET_DIR", &target)
        .env("CARGO_BUILD_JOBS", "16")
        .env("CARGO_INCREMENTAL", "0")
        .status()
        .unwrap();
    assert!(generated.success());
    let checked = Command::new(cargo)
        .args(["check", "--workspace", "--locked", "--offline", "-j", "16"])
        .current_dir(&scratch.0)
        .env("CARGO_TARGET_DIR", target)
        .env("CARGO_BUILD_JOBS", "16")
        .env("CARGO_INCREMENTAL", "0")
        .status()
        .unwrap();
    assert!(checked.success());
}

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    let mut entries = fs::read_dir(source)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let metadata = entry.file_type().unwrap();
        let destination = target.join(entry.file_name());
        if metadata.is_dir() {
            copy_tree(&entry.path(), &destination);
        } else if metadata.is_file() {
            fs::copy(entry.path(), destination).unwrap();
        } else {
            panic!("source tree contains a non-regular entry");
        }
    }
}
