use super::MutantCrate;

pub(super) fn assert_rejected(stderr: &str) {
    for (label, expression) in [
        ("resume", RESUME_EXPRESSION),
        ("recover", RECOVER_EXPRESSION),
        ("reconcile", RECONCILE_EXPRESSION),
    ] {
        assert!(
            stderr.contains("cannot find function `execute`") && stderr.contains(expression),
            "{label} failed for the wrong reason: {stderr}"
        );
    }
}

pub(super) fn expose(fixture: &MutantCrate) {
    for (path, addition) in [
        ("orchestration/product/resume.rs", RESUME_FIXTURE),
        ("orchestration/product/recover.rs", RECOVER_FIXTURE),
        ("orchestration/product/reconcile.rs", RECONCILE_FIXTURE),
    ] {
        let original = fixture.original(path);
        fixture.write_with(path, &original, addition);
    }
}

const RESUME_EXPRESSION: &str =
    "crate::orchestration::product::resume::execute(context, workspace, request)";
const RECOVER_EXPRESSION: &str =
    "crate::orchestration::product::recover::execute(context, workspace, request)";
const RECONCILE_EXPRESSION: &str =
    "crate::orchestration::product::reconcile::execute(context, workspace, request)";

pub(super) const MUTANTS: &str = r#"
mod exact_resume_route_mutant {
    fn probe(context: &crate::orchestration::product::ProductContext, workspace: &crate::orchestration::product::ProductWorkspace, request: &crate::orchestration::product::ResumeRequest) -> Result<crate::orchestration::product::ResumeOutcome, crate::orchestration::product::ProductError> {
        crate::orchestration::product::resume::execute(context, workspace, request)
    }
}
mod exact_recover_route_mutant {
    fn probe(context: &crate::orchestration::product::ProductContext, workspace: &crate::orchestration::product::ProductWorkspace, request: &crate::orchestration::product::RecoverRequest) -> Result<crate::orchestration::product::RecoverOutcome, crate::orchestration::product::ProductError> {
        crate::orchestration::product::recover::execute(context, workspace, request)
    }
}
mod exact_reconcile_route_mutant {
    fn probe(context: &crate::orchestration::product::ProductContext, workspace: &crate::orchestration::product::ProductWorkspace, request: &crate::orchestration::product::ReconcileRequest) -> Result<crate::orchestration::product::ReconcileOutcome, crate::orchestration::product::ProductError> {
        crate::orchestration::product::reconcile::execute(context, workspace, request)
    }
}
"#;

const RESUME_FIXTURE: &str = r#"
pub(crate) fn execute(_: &super::ProductContext, _: &super::ProductWorkspace, _: &ResumeRequest) -> Result<ResumeOutcome, super::ProductError> { Err(super::ProductError::AuthorityInvalid) }
"#;
const RECOVER_FIXTURE: &str = r#"
pub(crate) fn execute(_: &super::ProductContext, _: &super::ProductWorkspace, _: &RecoverRequest) -> Result<RecoverOutcome, super::ProductError> { Err(super::ProductError::AuthorityInvalid) }
"#;
const RECONCILE_FIXTURE: &str = r#"
pub(crate) fn execute(_: &super::ProductContext, _: &super::ProductWorkspace, _: &ReconcileRequest) -> Result<ReconcileOutcome, super::ProductError> { Err(super::ProductError::AuthorityInvalid) }
"#;
