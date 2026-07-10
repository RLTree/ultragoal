use std::path::Path;
use ultragoal::context::{BuildRequest, LiveContext};

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
