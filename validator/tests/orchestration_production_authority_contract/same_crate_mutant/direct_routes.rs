use super::MutantCrate;
use super::compile_cases::CaseSpec;

const PRODUCTION: &str = "orchestration/product/authority/production/mod.rs";

pub(super) const CASES: &[CaseSpec] = &[
    CaseSpec {
        marker: "N10_PRODUCT_RESUME_EXECUTE",
        file: PRODUCTION,
        code: "E0425",
        message: "cannot find function `execute`",
        additional_message: None,
    },
    CaseSpec {
        marker: "N10_PRODUCT_RECOVER_EXECUTE",
        file: PRODUCTION,
        code: "E0425",
        message: "cannot find function `execute`",
        additional_message: None,
    },
    CaseSpec {
        marker: "N10_PRODUCT_RECONCILE_EXECUTE",
        file: PRODUCTION,
        code: "E0425",
        message: "cannot find function `execute`",
        additional_message: None,
    },
];

pub(super) fn expose(fixture: &MutantCrate) {
    for (path, addition) in [
        ("orchestration/product/resume.rs", RESUME_EXPOSURE),
        ("orchestration/product/recover.rs", RECOVER_EXPOSURE),
        ("orchestration/product/reconcile.rs", RECONCILE_EXPOSURE),
    ] {
        fixture.append(path, addition);
    }
}

pub(super) const MUTANTS: &str = r#"
mod n10_product_direct_route_probes {
    fn resume(context: &crate::orchestration::product::ProductContext, workspace: &crate::orchestration::product::ProductWorkspace, request: &crate::orchestration::product::ResumeRequest) {
        let _ = crate::orchestration::product::resume::execute(context, workspace, request); // N10_PRODUCT_RESUME_EXECUTE
    }
    fn recover(context: &crate::orchestration::product::ProductContext, workspace: &crate::orchestration::product::ProductWorkspace, request: &crate::orchestration::product::RecoverRequest) {
        let _ = crate::orchestration::product::recover::execute(context, workspace, request); // N10_PRODUCT_RECOVER_EXECUTE
    }
    fn reconcile(context: &crate::orchestration::product::ProductContext, workspace: &crate::orchestration::product::ProductWorkspace, request: &crate::orchestration::product::ReconcileRequest) {
        let _ = crate::orchestration::product::reconcile::execute(context, workspace, request); // N10_PRODUCT_RECONCILE_EXECUTE
    }
}
"#;

const RESUME_EXPOSURE: &str = r#"
pub(crate) fn execute(_: &super::ProductContext, _: &super::ProductWorkspace, _: &ResumeRequest) -> Result<ResumeOutcome, super::ProductError> {
    Err(super::ProductError::AuthorityInvalid)
}
"#;
const RECOVER_EXPOSURE: &str = r#"
pub(crate) fn execute(_: &super::ProductContext, _: &super::ProductWorkspace, _: &RecoverRequest) -> Result<RecoverOutcome, super::ProductError> {
    Err(super::ProductError::AuthorityInvalid)
}
"#;
const RECONCILE_EXPOSURE: &str = r#"
pub(crate) fn execute(_: &super::ProductContext, _: &super::ProductWorkspace, _: &ReconcileRequest) -> Result<ReconcileOutcome, super::ProductError> {
    Err(super::ProductError::AuthorityInvalid)
}
"#;
