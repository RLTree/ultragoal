use syn::{Item, Visibility};

const TRANSACTION: &str =
    include_str!("../../src/routine_work/runtime_adapter/production/custody/transaction.rs");
const DURABLE: &str = include_str!(
    "../../src/routine_work/runtime_adapter/production/custody/transaction/durable_state.rs"
);
const INTENT: &str = include_str!(
    "../../src/routine_work/runtime_adapter/production/custody/transaction/intent_execution.rs"
);
const CUSTODY: &str =
    include_str!("../../src/routine_work/runtime_adapter/production/custody/mod.rs");
const PRODUCTION: &str = include_str!("../../src/routine_work/runtime_adapter/production/mod.rs");
const MEDIATOR: &str = include_str!("../../src/routine_work/runtime_adapter/mediator/mod.rs");
const OUTPUT: &str =
    include_str!("../../src/routine_work/runtime_adapter/production/output_journal/mod.rs");

#[test]
fn one_private_transaction_owns_reservation_and_terminal_authority() {
    assert_eq!(validate(TRANSACTION, DURABLE, INTENT, CUSTODY), Ok(()));
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
fn closest_sibling_topology_mutants_fail_closed() {
    let visible = TRANSACTION.replacen(
        "struct ReservationTransaction",
        "pub(super) struct ReservationTransaction",
        1,
    );
    assert_eq!(
        validate(&visible, DURABLE, INTENT, CUSTODY),
        Err("raw-owner-visible")
    );

    let redirected = TRANSACTION.replacen(
        "#[path = \"store/mod.rs\"]",
        "#[path = \"../output_journal/mod.rs\"]",
        1,
    );
    assert_eq!(
        validate(&redirected, DURABLE, INTENT, CUSTODY),
        Err("store-path-not-canonical")
    );

    let exported = CUSTODY.replacen(
        "pub(super) use transaction::mediate_reserved_effect;",
        "pub(crate) use transaction::*;",
        1,
    );
    assert_eq!(
        validate(TRANSACTION, DURABLE, INTENT, &exported),
        Err("transaction-export-broadened")
    );
}

fn validate(
    transaction: &str,
    durable: &str,
    intent: &str,
    custody: &str,
) -> Result<(), &'static str> {
    let syntax = syn::parse_file(transaction).map_err(|_| "transaction-parse")?;
    let owner = syntax
        .items
        .iter()
        .find_map(|item| match item {
            Item::Struct(item) if item.ident == "ReservationTransaction" => Some(item),
            _ => None,
        })
        .ok_or("owner-struct-missing")?;
    if !matches!(owner.vis, Visibility::Inherited)
        || owner
            .fields
            .iter()
            .any(|field| !matches!(field.vis, Visibility::Inherited))
    {
        return Err("raw-owner-visible");
    }
    if !transaction.contains("#[path = \"store/mod.rs\"]\nmod store;") {
        return Err("store-path-not-canonical");
    }
    let facade = "pub(super) use transaction::mediate_reserved_effect;\npub(super) use transaction::{\n    AuthorityBinding, OutputComponentJournal, OutputDirectoryIdentity, OutputProvisionJournal,\n    OutputStageAmbiguity,\n};";
    if custody.contains("pub(crate) use transaction")
        || custody.matches("pub(super) use transaction").count() != 2
        || !custody.contains(facade)
    {
        return Err("transaction-export-broadened");
    }
    for source in [durable, intent] {
        if source.contains("pub(crate) fn") || source.contains("pub(in crate") {
            return Err("owner-helper-export-broadened");
        }
    }
    Ok(())
}
