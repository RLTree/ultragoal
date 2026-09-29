use std::path::Path;
use ultragoal::context::{BuildRequest, ContextError, EffectClass, LiveContext};

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator package has a repository parent")
}

#[test]
fn accepted_live_candidate_builds_a_contract_bound_context() {
    let root = repository_root();
    let context = LiveContext::build(
        BuildRequest::new(root)
            .expect_repository_root(root)
            .expect_worktree_root(root)
            .select_input(
                "docs/ultragoal-contract-2026-07-successor-v2/FINAL-HANDOFF-MANIFEST.sha256",
            )
            .select_input("docs/ultragoal-successor-live/ADOPTION-AND-LIVE-BASELINE.json"),
    )
    .expect("live repository context builds");

    context
        .revalidate()
        .expect("live repository context remains current");
    assert_eq!(context.roots().repository_root, root.display().to_string());
    assert_eq!(context.roots().worktree_root, root.display().to_string());
    assert_eq!(context.selected_inputs().len(), 2);
    println!(
        "live_context_id={} head={} status_sha256={} untracked_sha256={}",
        context.context_id(),
        context
            .candidate()
            .head_commit
            .as_deref()
            .unwrap_or("unborn"),
        context.candidate().status_sha256,
        context.candidate().untracked_content_sha256,
    );
}

#[test]
fn public_callers_cannot_forge_non_read_effect_authority() {
    let root = repository_root();
    for effect in [
        EffectClass::PlannedWrite,
        EffectClass::WorkspaceWrite,
        EffectClass::ExternalWrite,
        EffectClass::Destructive,
    ] {
        assert!(matches!(
            LiveContext::build(BuildRequest::new(root).with_effect(effect)),
            Err(ContextError::EffectDenied(_))
        ));
    }
}

#[test]
fn read_context_rejects_effect_escalation_and_root_substitution() {
    let root = repository_root();
    let context = LiveContext::build(BuildRequest::new(root)).expect("read context builds");
    assert!(context.effect().authorize(EffectClass::Read).is_ok());
    assert!(matches!(
        context.effect().authorize(EffectClass::WorkspaceWrite),
        Err(ContextError::EffectDenied(_))
    ));
    assert!(matches!(
        LiveContext::build(BuildRequest::new(root).expect_worktree_root(root.join("validator"))),
        Err(ContextError::RootMismatch { .. })
    ));
}
