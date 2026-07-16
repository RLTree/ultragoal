use syn::{ImplItem, Item, Visibility};

const TRANSACTION: &str =
    include_str!("../../src/routine_work/runtime_adapter/production/reservation_transaction.rs");
const ISSUANCE: &str =
    include_str!("../../src/routine_work/runtime_adapter/production/production_issuance.rs");
const PRODUCTION: &str = include_str!("../../src/routine_work/runtime_adapter/production/mod.rs");
const MEDIATOR: &str = include_str!("../../src/routine_work/runtime_adapter/mediator/mod.rs");

#[test]
fn one_private_transaction_owns_reservation_and_terminal_authority() {
    assert_eq!(
        validate(TRANSACTION, ISSUANCE, PRODUCTION, MEDIATOR),
        Ok(())
    );
}

#[test]
fn authority_topology_mutants_fail_for_the_exact_boundary() {
    assert_eq!(
        validate(
            &TRANSACTION.replacen(
                "struct ReservationTransaction",
                "pub(super) struct ReservationTransaction",
                1,
            ),
            ISSUANCE,
            PRODUCTION,
            MEDIATOR,
        ),
        Err("raw-authority-visible")
    );
    assert_eq!(
        validate(
            &TRANSACTION.replacen(
                "owner: &'a ReservationTransaction",
                "pub(crate) owner: &'a ReservationTransaction",
                1,
            ),
            ISSUANCE,
            PRODUCTION,
            MEDIATOR,
        ),
        Err("capability-owner-visible")
    );
    let terminal_capability = TRANSACTION.replacen(
        "impl RoutineExecutionCapability<'_> {",
        "impl RoutineExecutionCapability<'_> {\n    pub(crate) fn settle(&self) {}",
        1,
    );
    assert_eq!(
        validate(&terminal_capability, ISSUANCE, PRODUCTION, MEDIATOR),
        Err("capability-surface")
    );
    let split_issuer = format!("{ISSUANCE}\nfn bypass() {{ ledger.reserve(spec); }}\n");
    assert_eq!(
        validate(TRANSACTION, &split_issuer, PRODUCTION, MEDIATOR),
        Err("parallel-ledger-authority")
    );
    assert_eq!(
        validate(
            &TRANSACTION.replacen("owner.ledger.validate_reserved(&owner.token)?;", "", 1,),
            ISSUANCE,
            PRODUCTION,
            MEDIATOR,
        ),
        Err("reserved-token-validation")
    );
}

fn validate(
    transaction: &str,
    issuance: &str,
    production: &str,
    mediator: &str,
) -> Result<(), &'static str> {
    let syntax = syn::parse_file(transaction).map_err(|_| "transaction-parse")?;
    let owner = struct_item(&syntax, "ReservationTransaction")?;
    if !matches!(owner.vis, Visibility::Inherited)
        || owner
            .fields
            .iter()
            .any(|field| !matches!(field.vis, Visibility::Inherited))
    {
        return Err("raw-authority-visible");
    }
    let capability = struct_item(&syntax, "RoutineExecutionCapability")?;
    if !matches!(capability.vis, Visibility::Restricted(_))
        || capability
            .fields
            .iter()
            .any(|field| !matches!(field.vis, Visibility::Inherited))
    {
        return Err("capability-owner-visible");
    }
    let mut methods = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Impl(item) if type_name(&item.self_ty, "RoutineExecutionCapability") => {
                Some(item)
            }
            _ => None,
        })
        .flat_map(|item| item.items.iter())
        .filter_map(|item| match item {
            ImplItem::Fn(method) => Some(method.sig.ident.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    methods.sort();
    if methods
        != [
            "authenticates_artifact",
            "observe_staged_transition",
            "prepare_spawn",
            "reuse_only",
            "stage_and_use",
        ]
    {
        return Err("capability-surface");
    }
    for operation in [
        "owner.ledger.validate_reserved(&owner.token)?;",
        "ledger.reserve(spec)?;",
        "ledger.pending_recovery(&binding)?",
        ".stage_success(&owner.token, &authenticated)?;",
        "self.ledger.settle(&self.token, state, artifacts)?;",
        "self.ledger.record_failure(&self.token, evidence)",
    ] {
        if !transaction.contains(operation) {
            return Err("reserved-token-validation");
        }
    }
    if [
        "FileAuthorityLedger",
        "ReservationToken",
        ".reserve(",
        ".settle(",
    ]
    .iter()
    .any(|operation| issuance.contains(operation))
    {
        return Err("parallel-ledger-authority");
    }
    if !production.contains("reservation_transaction::mediate_reserved_effect(")
        || production.contains("ProductionRoutineIssuer")
        || mediator.contains("RoutineRootGrant")
        || mediator.contains("run_reserved")
        || mediator.contains("ReservationTerminal")
    {
        return Err("canonical-route-topology");
    }
    Ok(())
}

fn struct_item<'a>(file: &'a syn::File, name: &str) -> Result<&'a syn::ItemStruct, &'static str> {
    file.items
        .iter()
        .find_map(|item| match item {
            Item::Struct(item) if item.ident == name => Some(item),
            _ => None,
        })
        .ok_or("owner-struct-missing")
}

fn type_name(item: &syn::Type, name: &str) -> bool {
    let syn::Type::Path(path) = item else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == name)
}
