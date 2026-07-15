use std::collections::BTreeMap;

use syn::visit::Visit;

#[path = "reservation_authority_shape/custody_receiver_identity.rs"]
mod custody_receiver_identity;
#[path = "reservation_authority_shape/custody_syntax.rs"]
mod custody_syntax;
#[path = "reservation_authority_shape/declarations.rs"]
mod declarations;
#[path = "reservation_authority_shape/operations.rs"]
mod operations;
#[path = "reservation_authority_shape/pattern_aliases.rs"]
mod pattern_aliases;

use operations::BodyShape;

const AUTHORITY_SOURCE: &str =
    include_str!("../../src/routine_work/runtime_adapter/mediator/reservation_state/authority.rs");
const STAGED_SOURCE: &str = include_str!(
    "../../src/routine_work/runtime_adapter/mediator/reservation_state/staged_custody.rs"
);

pub(super) fn assert_current() {
    let graph =
        include_str!("../../src/routine_work/runtime_adapter/mediator/reservation_state/mod.rs");
    assert!(graph.contains("mod authority;"));
    assert!(graph.contains("pub(super) use authority::{AttemptReservation, reserve_grant};"));
    assert!(!graph.contains("reserved_test_attempt"));
    assert_eq!(validate(AUTHORITY_SOURCE, STAGED_SOURCE), Ok(()));
}

pub(super) fn validate(authority: &str, staged: &str) -> Result<(), &'static str> {
    validate_authority(authority)?;
    validate_staged(staged)
}

fn validate_authority(source: &str) -> Result<(), &'static str> {
    let file = syn::parse_file(source).map_err(|_| "authority-parse")?;
    declarations::validate_authority(&file)?;

    let implementations = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item) => Some(item),
            _ => None,
        })
        .collect::<Vec<_>>();
    if implementations.len() != 1 || implementations[0].trait_.is_some() {
        return Err("authority-impl-shape");
    }
    let methods = methods(implementations[0])?;
    if methods != expected_authority_methods() {
        return Err("authority-methods");
    }

    let mut shape = BodyShape::new(
        ["started", "settled", "staged"],
        ["AttemptReservation", "Self"],
    );
    shape.visit_file(&file);
    shape.require_plain(file.items.len())?;
    shape.require_no_custody_patterns()?;
    shape.require_direct_transitions()?;
    shape.require_identity_bound()?;
    shape.require_bound_operations(expected_authority_operations())?;
    shape.require_sensitive_calls([
        "settle_incomplete:self:finish_terminal",
        "settle_success:self:finish_terminal",
    ])?;
    Ok(())
}

fn validate_staged(source: &str) -> Result<(), &'static str> {
    let file = syn::parse_file(source).map_err(|_| "staged-parse")?;
    declarations::validate_staged(&file)?;
    let implementations = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item) => Some(item),
            _ => None,
        })
        .collect::<Vec<_>>();
    if implementations.len() != 1 || implementations[0].trait_.is_some() {
        return Err("staged-impl-shape");
    }
    let expected = [
        "cleanup_last",
        "clear_recorded",
        "failure_transfer_required",
        "is_empty",
        "new",
        "push_and_use",
        "require_empty",
    ];
    let methods = methods(implementations[0])?;
    if methods.keys().cloned().collect::<Vec<_>>() != expected
        || methods.values().any(|visibility| visibility != "super")
    {
        return Err("staged-methods");
    }
    let mut shape = BodyShape::new(["0"], ["StagedCustody", "Self"]);
    shape.visit_file(&file);
    shape.require_plain(file.items.len())?;
    shape.require_no_custody_patterns()?;
    shape.require_direct_transitions()?;
    shape.require_identity_bound()?;
    shape.require_bound_operations([
        "cleanup_last:0:borrow",
        "cleanup_last:0:borrow_mut",
        "clear_recorded:0:borrow_mut",
        "is_empty:0:borrow",
        "push_and_use:0:borrow",
        "push_and_use:0:borrow_mut",
    ])?;
    shape.require_sensitive_calls([
        "failure_transfer_required:self:is_empty",
        "require_empty:self:is_empty",
    ])?;
    Ok(())
}

fn methods(item: &syn::ItemImpl) -> Result<BTreeMap<String, String>, &'static str> {
    if !item.attrs.is_empty() {
        return Err("authority-impl-attributes");
    }
    item.items
        .iter()
        .map(|member| {
            let syn::ImplItem::Fn(method) = member else {
                return Err("authority-impl-member-kind");
            };
            let receiver = method.sig.receiver();
            if method.sig.ident != "new"
                && !receiver
                    .is_some_and(|item| item.reference.is_some() && item.mutability.is_none())
            {
                return Err("authority-method-receiver");
            }
            Ok((method.sig.ident.to_string(), visibility(&method.vis)))
        })
        .collect()
}

fn expected_authority_methods() -> BTreeMap<String, String> {
    let double = [
        "authenticates_artifact",
        "grant_id",
        "is_started",
        "mark_started",
        "prepare_spawn",
        "protocol_id",
        "recovery_marker",
        "retain_non_durable_authentication",
        "reuse_only",
        "settle_incomplete",
        "settle_success",
        "stage_and_use",
        "stage_success",
    ];
    let single = [
        "cleanup_staged",
        "failure_evidence",
        "record_failure_and_transition",
        "terminal_is_authoritative",
    ];
    double
        .into_iter()
        .map(|name| (name.to_owned(), "super::super".to_owned()))
        .chain(
            single
                .into_iter()
                .map(|name| (name.to_owned(), "super".to_owned())),
        )
        .chain([
            ("finish_terminal".to_owned(), "private".to_owned()),
            ("require_open".to_owned(), "private".to_owned()),
        ])
        .collect()
}

fn expected_authority_operations() -> [&'static str; 21] {
    [
        "cleanup_staged:staged:cleanup_last",
        "cleanup_staged:staged:is_empty",
        "failure_evidence:started:get",
        "finish_terminal:settled:set(true)",
        "finish_terminal:started:get",
        "is_started:started:get",
        "mark_started:started:set(true)",
        "record_failure_and_transition:settled:set(true)",
        "record_failure_and_transition:staged:clear_recorded",
        "record_failure_and_transition:staged:failure_transfer_required",
        "record_failure_and_transition:started:get",
        "record_failure_and_transition:started:get",
        "require_open:settled:get",
        "settle_incomplete:staged:require_empty",
        "settle_success:staged:require_empty",
        "stage_and_use:staged:push_and_use",
        "stage_and_use:staged:require_empty",
        "stage_and_use:staged:require_empty",
        "stage_success:staged:require_empty",
        "terminal_is_authoritative:settled:get",
        "terminal_is_authoritative:staged:is_empty",
    ]
}

fn visibility(value: &syn::Visibility) -> String {
    match value {
        syn::Visibility::Inherited => "private".to_owned(),
        syn::Visibility::Restricted(item) => item
            .path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        syn::Visibility::Public(_) => "public".to_owned(),
    }
}
