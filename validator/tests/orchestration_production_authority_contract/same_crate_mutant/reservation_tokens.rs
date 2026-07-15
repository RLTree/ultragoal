use super::MutantCrate;

const ROUTE: &str = "orchestration/product/authority/production/execution_transaction/route.rs";

pub(super) fn install_setup(fixture: &MutantCrate) {
    let source = fixture.original(ROUTE);
    fixture.write_with(
        ROUTE,
        &source,
        r#"pub(in crate::orchestration::product::authority::production::sealed_authority::execution_transaction)
fn mutant_request() -> ExecutionRequest<'static> {
    unsafe { std::mem::MaybeUninit::zeroed().assume_init() }
}
pub(in crate::orchestration::product::authority::production::sealed_authority::execution_transaction)
fn mutant_validated() -> ValidatedExecution<'static> {
    ValidatedExecution { permit_id: String::new(), request: mutant_request() }
}
pub(in crate::orchestration::product::authority::production::sealed_authority::execution_transaction)
fn mutant_reserved() -> ReservedExecution<'static> {
    ReservedExecution { permit_id: String::new(), request: mutant_request() }
}"#,
    );
}

pub(super) fn assert_types_rejected(stderr: &str) {
    for (label, expected, expression) in [
        (
            "validated construction",
            "struct `ValidatedExecution` is private",
            "super::route::ValidatedExecution {",
        ),
        (
            "reserved construction",
            "struct `ReservedExecution` is private",
            "super::route::ReservedExecution {",
        ),
    ] {
        assert!(
            stderr.contains(expected) && stderr.contains(expression),
            "{label} type boundary failed for the wrong reason: {stderr}"
        );
    }
}

pub(super) fn assert_methods_rejected(stderr: &str) {
    for (label, expected, expression) in [
        (
            "validated identity",
            "method `permit_id` is private",
            "mutant_validated(); let _ = token.permit_id()",
        ),
        (
            "validated reservation",
            "method `reserve` is private",
            "mutant_validated(); let _ = token.reserve()",
        ),
        (
            "reserved identity",
            "method `permit_id` is private",
            "mutant_reserved(); let _ = token.permit_id()",
        ),
        (
            "reserved execution",
            "method `execute` is private",
            "mutant_reserved(); let _ = token.execute()",
        ),
    ] {
        assert!(
            stderr.contains(expected) && stderr.contains(expression),
            "{label} failed for the wrong reason: {stderr}"
        );
    }
}

pub(super) fn expose_types(fixture: &MutantCrate) {
    let source = fixture.original(ROUTE);
    let scope = "pub(in crate::orchestration::product::authority::production::sealed_authority::execution_transaction) ";
    let exposed = source
        .replace(
            "enum ExecutionRequest<'a>",
            &format!("{scope}enum ExecutionRequest<'a>"),
        )
        .replace(
            "struct ValidatedExecution<'a>",
            &format!("{scope}struct ValidatedExecution<'a>"),
        )
        .replace(
            "struct ReservedExecution<'a>",
            &format!("{scope}struct ReservedExecution<'a>"),
        );
    std::fs::write(fixture.source(ROUTE), exposed).unwrap();
}

pub(super) fn expose_members(fixture: &MutantCrate) {
    let source = fixture.original(ROUTE);
    let scope = "pub(in crate::orchestration::product::authority::production::sealed_authority::execution_transaction) ";
    let exposed = source
        .replace(
            "    permit_id: String,",
            &format!("    {scope}permit_id: String,"),
        )
        .replace(
            "    request: ExecutionRequest<'a>,",
            &format!("    {scope}request: ExecutionRequest<'a>,"),
        )
        .replace("    fn permit_id(", &format!("    {scope}fn permit_id("))
        .replace("    fn reserve(", &format!("    {scope}fn reserve("))
        .replace("    fn execute(", &format!("    {scope}fn execute("));
    std::fs::write(fixture.source(ROUTE), exposed).unwrap();
}

pub(super) const MUTANTS: &str = r#"
mod validated_construction_mutant {
    fn probe() -> super::route::ValidatedExecution<'static> {
        let request = super::route::mutant_request();
        super::route::ValidatedExecution { permit_id: String::new(), request }
    }
}
mod reserved_construction_mutant {
    fn probe() -> super::route::ReservedExecution<'static> {
        let request = super::route::mutant_request();
        super::route::ReservedExecution { permit_id: String::new(), request }
    }
}
mod validated_identity_mutant {
    fn probe() { let token = super::route::mutant_validated(); let _ = token.permit_id(); }
}
mod validated_reservation_mutant {
    fn probe() { let token = super::route::mutant_validated(); let _ = token.reserve(); }
}
mod reserved_identity_mutant {
    fn probe() { let token = super::route::mutant_reserved(); let _ = token.permit_id(); }
}
mod reserved_execution_mutant {
    fn probe() { let token = super::route::mutant_reserved(); let _ = token.execute(); }
}
"#;
