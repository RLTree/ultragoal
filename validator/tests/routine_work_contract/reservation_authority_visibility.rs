use syn::{Item, Visibility};
use walkdir::WalkDir;

use super::owned_compile_scratch::{OwnedCompileScratch, configured_root};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const TRANSACTION: &str =
    include_str!("../../src/routine_work/runtime_adapter/production/custody/transaction.rs");
const OWNER: &str =
    include_str!("../../src/routine_work/runtime_adapter/production/custody/transaction/owner.rs");
const RECORDS: &str = include_str!(
    "../../src/routine_work/runtime_adapter/production/custody/store/authority_record.rs"
);
const CUSTODY: &str =
    include_str!("../../src/routine_work/runtime_adapter/production/custody/mod.rs");
const PRODUCTION: &str = include_str!("../../src/routine_work/runtime_adapter/production/mod.rs");
const MEDIATOR: &str = include_str!("../../src/routine_work/runtime_adapter/mediator/mod.rs");
const OUTPUT: &str =
    include_str!("../../src/routine_work/runtime_adapter/production/output_journal/mod.rs");
#[test]
fn one_childless_owner_holds_reservation_and_terminal_authority() {
    assert_eq!(validate(TRANSACTION, OWNER, RECORDS, CUSTODY), Ok(()));
    assert!(PRODUCTION.contains("custody::mediate_reserved_effect("));
    for source in [PRODUCTION, MEDIATOR, OUTPUT] {
        assert!(!source.contains("ReservationToken"));
        assert!(!source.contains("FileAuthorityLedger"));
        assert!(!source.contains("DurableWrite"));
        assert!(!source.contains("custody::store"));
        assert!(!source.contains("custody::transaction"));
    }
}
#[test]
fn child_and_raw_record_visibility_mutants_fail_closed() {
    let visible = OWNER.replacen(
        "pub(super) struct ReservationOwner",
        "pub(crate) struct ReservationOwner",
        1,
    );
    assert_eq!(
        validate(TRANSACTION, &visible, RECORDS, CUSTODY),
        Err("raw-owner-visible")
    );
    let child = format!("{OWNER}\nmod descendant {{}}\n");
    assert_eq!(
        validate(TRANSACTION, &child, RECORDS, CUSTODY),
        Err("owner-has-descendant")
    );
    let transaction_child = format!("{TRANSACTION}\nmod descendant {{}}\n");
    assert_eq!(
        validate(&transaction_child, OWNER, RECORDS, CUSTODY),
        Err("transaction-has-descendant")
    );
    let records = RECORDS.replacen(
        "pub(super) struct ChildLease",
        "pub(crate) struct ChildLease",
        1,
    );
    assert_eq!(
        validate(TRANSACTION, OWNER, &records, CUSTODY),
        Err("raw-record-visible")
    );
}
#[test]
fn exact_custody_sibling_cannot_construct_clone_or_forge_raw_authority() {
    let mut scratch = OwnedCompileScratch::claim("routine-custody-privacy");
    let result = (|| {
        let copied = scratch.path().join("validator");
        copy_validator(&copied)?;
        let open = compile(&copied);
        if !open.status.success() {
            return Err(format!("opened production graph failed: {}", stderr(&open)));
        }
        let custody = copied.join("src/routine_work/runtime_adapter/production/custody");
        fs::write(custody.join("privacy_attack.rs"), ATTACK).map_err(|e| e.to_string())?;
        let module = custody.join("mod.rs");
        let mut source = fs::read_to_string(&module).map_err(|e| e.to_string())?;
        source.push_str("\nmod privacy_attack;\n");
        fs::write(module, source).map_err(|e| e.to_string())?;
        let closed = compile(&copied);
        let diagnostic = stderr(&closed);
        if closed.status.success()
            || ![
                "ReservationToken",
                "ChildLease",
                "LaunchStageRecord",
                "TerminalRecord",
                "LaunchCleanupEvidence",
                "RoutineError",
                "clone_launch_cleanup",
                "clone_routine_error",
                "no method named `clone`",
            ]
            .into_iter()
            .all(|name| diagnostic.contains(name))
            || !diagnostic.contains("private")
            || !diagnostic.contains("FileLedger")
            || !diagnostic.contains("DurableCustody")
            || !diagnostic.contains("ReservationSpec")
            || !diagnostic.contains("privacy_attack.rs")
        {
            return Err(format!(
                "privacy attack was not causally rejected: {diagnostic}"
            ));
        }
        Ok(())
    })();
    scratch.teardown_after_assertions();
    assert_eq!(result, Ok(()));
}
const ATTACK: &str = r#"
use crate::routine_work::{runtime_adapter::LaunchCleanupEvidence, RoutineError};
use super::store::{ChildLease, LaunchStageRecord, ReservationToken, TerminalRecord};
use super::store::DurableCustody;
use super::store::supported::record_authentication::FileLedger;
use super::transaction::ReservationSpec;
fn raw_types() {
    let _: (Option<ReservationToken>, Option<ChildLease>, Option<LaunchStageRecord>, Option<TerminalRecord>) = (None, None, None, None);
}
fn clone_custody(value: DurableCustody) { let _ = value.clone(); }
fn clone_launch_cleanup(value: LaunchCleanupEvidence) { let _ = value.clone(); }
fn clone_routine_error(value: RoutineError) { let _ = value.clone(); }
fn raw_ledger(value: FileLedger) { let _ = value; }
fn construct_custody() { let _ = DurableCustody {}; }
fn reconstruct_spec(seed: ReservationSpec) {
    let _ = ReservationSpec {
        binding: seed.binding,
        request_id: seed.request_id,
        grant_id: seed.grant_id,
        recovery_marker: seed.recovery_marker,
        owner: seed.owner,
        output_journal: seed.output_journal,
        intents: seed.intents,
    };
}
"#;
fn copy_validator(destination: &Path) -> Result<(), String> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    copy_tree(source, destination)?;
    let manifest = fs::read_to_string(source.join("Cargo.toml"))
        .map_err(|e| e.to_string())?
        .replace("[lints]\nworkspace = true\n", "")
        + "\n[workspace]\n";
    fs::write(destination.join("Cargo.toml"), manifest).map_err(|e| e.to_string())?;
    let repository = source.parent().ok_or("validator parent missing")?;
    fs::copy(
        repository.join("plugin-manifest-draft.json"),
        destination
            .parent()
            .unwrap()
            .join("plugin-manifest-draft.json"),
    )
    .map_err(|e| e.to_string())?;
    copy_tree(
        &repository.join("templates"),
        &destination.parent().unwrap().join("templates"),
    )?;
    copy_tree(
        &repository.join(".codex/agents"),
        &destination.parent().unwrap().join(".codex/agents"),
    )?;
    copy_tree(
        &repository.join("docs/ultragoal-contract-2026-07-successor-v2"),
        &destination
            .parent()
            .unwrap()
            .join("docs/ultragoal-contract-2026-07-successor-v2"),
    )
}
fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    for entry in WalkDir::new(source) {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let relative = path.strip_prefix(source).map_err(|e| e.to_string())?;
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(target).map_err(|e| e.to_string())?;
        } else {
            fs::copy(entry.path(), target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
fn compile(root: &Path) -> Output {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--quiet", "--lib"])
        .current_dir(root)
        .env("CARGO_TARGET_DIR", configured_root("CARGO_TARGET_DIR"))
        .env("RUSTFLAGS", "--cap-lints allow")
        .output()
        .unwrap()
}
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
fn validate(
    transaction: &str,
    owner: &str,
    records: &str,
    custody: &str,
) -> Result<(), &'static str> {
    let syntax = syn::parse_file(owner).map_err(|_| "owner-parse")?;
    let owner = syntax
        .items
        .iter()
        .find_map(|item| match item {
            Item::Struct(item) if item.ident == "ReservationOwner" => Some(item),
            _ => None,
        })
        .ok_or("owner-struct-missing")?;
    if !matches!(&owner.vis, Visibility::Restricted(value) if value.path.is_ident("super"))
        || owner
            .fields
            .iter()
            .any(|field| !matches!(field.vis, Visibility::Inherited))
    {
        return Err("raw-owner-visible");
    }
    if syntax.items.iter().any(|item| matches!(item, Item::Mod(_))) {
        return Err("owner-has-descendant");
    }
    let transaction_syntax = syn::parse_file(transaction).map_err(|_| "transaction-parse")?;
    let children = transaction_syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(item) => Some(item.ident.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if children != ["owner"] {
        return Err("transaction-has-descendant");
    }
    if !custody.contains("#[path = \"store/mod.rs\"]\nmod store;") {
        return Err("owner-topology-invalid");
    }
    for name in [
        "ChildLease",
        "LaunchStageRecord",
        "TerminalRecord",
        "ReservationToken",
    ] {
        if records.contains(&format!("pub(crate) struct {name}")) {
            return Err("raw-record-visible");
        }
    }
    Ok(())
}
