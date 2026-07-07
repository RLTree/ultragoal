use crate::audit::contract::Failure;
use serde_json::json;

fn has_error(out: &[Failure], error: &str) -> bool {
    out.iter().any(|failure| failure.error == error)
}

#[test]
fn coverage_policy_rejects_check_and_test_substitutions_only_for_completion_claims() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-trigger-policy");
    let mut out = Vec::new();
    crate::claim_semantics::coverage::policy::check(
        &json!({
            "id":"COV-CI",
            "title":"All checks passed with material sign-off",
            "description":"The package is ready for release advancement without coverage evidence."
        }),
        &root,
        &mut out,
    );
    assert!(
        has_error(&out, "coverage_check_pass_substitution"),
        "{out:?}"
    );

    out.clear();
    crate::claim_semantics::coverage::policy::check(
        &json!({
            "id":"COV-FIXTURE",
            "title":"Target fixture passed and release-ready",
            "description":"Generated examples passed, but no coverage receipt exists."
        }),
        &root,
        &mut out,
    );
    assert!(
        has_error(&out, "coverage_test_pass_substitution"),
        "{out:?}"
    );

    out.clear();
    crate::claim_semantics::coverage::policy::check(
        &json!({
            "id":"COV-NON-COMPLETION",
            "title":"All checks passed",
            "description":"Informational status only."
        }),
        &root,
        &mut out,
    );
    assert!(out.is_empty(), "{out:?}");
    let _ = std::fs::remove_dir_all(root);
}
