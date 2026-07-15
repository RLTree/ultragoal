use super::fixture::TestRoot;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn descendants_cannot_mint_authority_or_call_direct_effects() {
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
    for path in [
        "templates",
        ".codex/agents",
        "docs/ultragoal-contract-2026-07-successor-v2",
    ] {
        copy_tree(&repository_root.join(path), &scratch.path().join(path));
    }
    copy_required_manifest_files(repository_root, &crate_root, scratch.path());

    let production = crate_root.join("src/orchestration/product/authority/production/mod.rs");
    append_mutant(&production, PRODUCTION_DESCENDANT_MUTANT);
    let transaction =
        crate_root.join("src/orchestration/product/authority/production/execution_transaction.rs");
    append_mutant(&transaction, TRANSACTION_DESCENDANT_MUTANT);

    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--lib", "--quiet"])
        .current_dir(&crate_root)
        .env("CARGO_TARGET_DIR", crate_root.join("target"))
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "descendant mutants compiled");
    for (label, expected) in [
        ("raw store", "module `store` is private"),
        ("raw ledger", "module `ledger` is private"),
        ("raw signer", "module `root_authority` is private"),
        ("sealed constructor", "associated function `new` is private"),
        ("sealed fields", "field `authority` of struct"),
        ("sealed clone", "no method named `clone`"),
        (
            "reservation construction",
            "module `execution_transaction` is private",
        ),
        ("resume effect", "module `resume` is private"),
        ("recover effect", "module `recover` is private"),
        ("reconcile effect", "module `reconcile` is private"),
        (
            "direct resume route",
            "expected function, found module `crate::orchestration::product::resume`",
        ),
        (
            "direct recover route",
            "expected function, found module `crate::orchestration::product::recover`",
        ),
        (
            "direct reconcile route",
            "expected function, found module `crate::orchestration::product::reconcile`",
        ),
    ] {
        assert!(
            stderr.contains(expected),
            "{label} failed for the wrong reason: {stderr}"
        );
    }
}

const PRODUCTION_DESCENDANT_MUTANT: &str = r#"
mod descendant_authority_mutant {
    use crate::orchestration::product::{PermitTarget, ProductContext, ProductWorkspace, ProductionRootAuthority, ReconcileRequest, RecoverRequest, ResumeRequest, RootOperation};
    use crate::orchestration::{Actor, Binding};
    use std::path::Path;

    fn forge(root: &Path, actor: Actor) {
        let (store, _) = super::sealed_authority::store::Store::open_or_initialize(root, actor.as_str()).unwrap();
        let ledger = super::sealed_authority::ledger::Ledger::open(store).unwrap();
        let _authority = super::sealed_authority::ProductionRootAuthority::new(actor, [0; 32], ledger);
    }

    fn raw_issue(root: &Path, actor: Actor, permit_id: &str, slot_id: &str) {
        let signer = super::sealed_authority::root_authority::RootAuthority { root_actor: actor.clone(), key: [0; 32] };
        let (store, _) = super::sealed_authority::store::Store::open_or_initialize(root, actor.as_str()).unwrap();
        let ledger = super::sealed_authority::ledger::Ledger::open(store).unwrap();
        let _permit = signer.issue(super::sealed_authority::root_authority::RootPermitIssuance {
            operation: RootOperation::Resume,
            binding: Binding::new(permit_id, slot_id).unwrap(),
            workspace_identity: permit_id,
            journal_head_identity: slot_id,
            issued_tick: 1,
            expires_tick: u64::MAX,
            nonce: b"attacker-known-nonce-0123456789",
            target: PermitTarget::default(),
            decision_binding: panic!(),
        }).unwrap();
        ledger.issue(permit_id, slot_id).unwrap();
    }

    fn extract(authority: ProductionRootAuthority) {
        let _ = &authority.authority;
        let _ = authority.clone();
    }

    fn forge_reservation() {
        let _ = super::sealed_authority::execution_transaction::route::ValidatedExecution {
            permit_id: String::new(),
            request: panic!(),
        };
    }

    fn direct_routes(context: &ProductContext, workspace: &ProductWorkspace, resume: &ResumeRequest, recover: &RecoverRequest, reconcile: &ReconcileRequest) {
        let _ = crate::orchestration::product::resume(context, workspace, resume);
        let _ = crate::orchestration::product::recover(context, workspace, recover);
        let _ = crate::orchestration::product::reconcile(context, workspace, reconcile);
    }
}
"#;

const TRANSACTION_DESCENDANT_MUTANT: &str = r#"
mod descendant_effect_mutant {
    use crate::orchestration::product::{ProductContext, ProductWorkspace, ReconcileRequest, RecoverRequest, ResumeRequest};

    fn direct_effects(context: &ProductContext, workspace: &ProductWorkspace, resume: &ResumeRequest, recover: &RecoverRequest, reconcile: &ReconcileRequest) {
        let _ = super::route::resume::execute(context, workspace, resume);
        let _ = super::route::recover::execute(context, workspace, recover);
        let _ = super::route::reconcile::execute(context, workspace, reconcile);
    }
}
"#;

fn append_mutant(path: &Path, mutant: &str) {
    let source = fs::read_to_string(path).unwrap();
    fs::write(path, format!("{source}\n{mutant}\n")).unwrap();
}

fn copy_required_manifest_files(repository_root: &Path, crate_root: &Path, scratch: &Path) {
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
        scratch.join("plugin-manifest-draft.json"),
    )
    .unwrap();
    let manifest = fs::read_to_string(repository_root.join("validator/Cargo.toml"))
        .unwrap()
        .replace("[lints]\nworkspace = true\n\n", "")
        + "\n[workspace]\n";
    fs::write(crate_root.join("Cargo.toml"), manifest).unwrap();
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
