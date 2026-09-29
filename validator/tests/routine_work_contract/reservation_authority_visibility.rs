use super::issuer_api_compilation;
use super::owned_compile_scratch::OwnedCompileScratch;
use syn::{Item, Visibility};

const TRANSACTION: &str =
    include_str!("../../src/routine_work/runtime_adapter/production/custody/transaction/mod.rs");
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
fn public_boundary_refuses_raw_custody_authority() {
    let mut scratch = OwnedCompileScratch::claim("routine-custody-probe");
    let output = issuer_api_compilation::check(scratch.path(), "production_raw_custody_consumer");
    assert!(
        !output.status.success(),
        "raw custody authority became public"
    );
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("E0603"), "{diagnostic}");
    scratch.teardown_after_assertions();
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
