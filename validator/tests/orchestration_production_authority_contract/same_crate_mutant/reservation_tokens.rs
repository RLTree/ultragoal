use super::MutantCrate;

const ROUTE: &str = "orchestration/product/authority/production/execution_transaction/route.rs";

pub(super) fn expose_types(fixture: &MutantCrate) {
    let scope = "pub(in crate::orchestration::product::authority::production::sealed_authority::execution_transaction) ";
    for name in [
        "ExecutionRequest",
        "ValidatedExecution",
        "ReservedExecution",
    ] {
        let kind = if name == "ExecutionRequest" {
            "enum"
        } else {
            "struct"
        };
        fixture.replace(
            ROUTE,
            &format!("{kind} {name}<'a>"),
            &format!("{scope}{kind} {name}<'a>"),
        );
    }
}

pub(super) fn expose_members(fixture: &MutantCrate) {
    let scope = "pub(in crate::orchestration::product::authority::production::sealed_authority::execution_transaction) ";
    fixture.replace(
        ROUTE,
        "    permit_id: String,",
        &format!("    {scope}permit_id: String,"),
    );
    fixture.replace(
        ROUTE,
        "    request: ExecutionRequest<'a>,",
        &format!("    {scope}request: ExecutionRequest<'a>,"),
    );
    for method in ["permit_id", "reserve", "execute"] {
        fixture.replace(
            ROUTE,
            &format!("    fn {method}("),
            &format!("    {scope}fn {method}("),
        );
    }
    fixture.append(
        ROUTE,
        &format!(
            "impl ValidatedExecution<'_> {{ {scope}fn clone(&self) {{}} }}\nimpl ReservedExecution<'_> {{ {scope}fn clone(&self) {{}} }}"
        ),
    );
}
