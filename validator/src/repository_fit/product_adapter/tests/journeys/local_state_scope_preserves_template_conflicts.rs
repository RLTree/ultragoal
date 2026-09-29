use super::*;
use crate::repository_fit::product_adapter::{
    FitPlanScope, plan_target_for_scope, prepare_apply_request,
};

#[test]
pub(crate) fn local_state_scope_changes_only_the_required_ignore_rule() {
    let fixture = Fixture::new("local-state-scope");
    fixture.write("AGENTS.md", b"user-owned conflict\n");
    fixture.write(".gitignore", b"/target/\n");
    fixture.write("operator-state.txt", b"preserve exactly\n");
    let context = fixture.context();
    let record = assert_zero_write(&fixture, || {
        plan_target_for_scope(&context, FitPlanScope::LocalState).unwrap()
    });
    assert_eq!(record.scope, FitPlanScope::LocalState);
    assert_eq!(record.desired.file_count, 0);
    assert!(record.plan.mutations.is_empty());
    assert!(record.plan.conflicts.is_empty());
    assert!(record.plan.local_state.as_ref().unwrap().mutation_required);

    let prepared = prepare_apply_request(
        &context,
        &record.to_machine_bytes().unwrap(),
        record.plan_sha256(),
    )
    .unwrap();
    let verification = execute(&fixture, prepared).unwrap();
    assert!(verification.idempotent());
    assert_eq!(verification.matched_files(), 0);
    assert_eq!(
        fs::read(fixture.root.join("AGENTS.md")).unwrap(),
        b"user-owned conflict\n"
    );
    assert_eq!(
        fs::read(fixture.root.join(".gitignore")).unwrap(),
        b"/target/\nvalidation_artifacts/\n"
    );
    assert_eq!(
        fs::read(fixture.root.join("operator-state.txt")).unwrap(),
        b"preserve exactly\n"
    );
}
