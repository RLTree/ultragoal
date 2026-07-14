use super::super::capture::{CommandSpec, PublicArg};
use super::super::fixture::RepoFixture;
use crate::context::{BuildRequest, LiveContext};

fn catalog_context(fixture: &RepoFixture) -> LiveContext {
    LiveContext::build(
        BuildRequest::new(fixture.root())
            .expect_repository_root(fixture.root())
            .expect_worktree_root(fixture.root())
            .probe_tool("sandbox-exec")
            .probe_tool("printf"),
    )
    .unwrap()
}

#[test]
fn finite_successful_output_cannot_race_past_the_combined_observation_limit() {
    let fixture = RepoFixture::new("finite-output-race");
    let context = catalog_context(&fixture);
    let payload = "x".repeat(4096);

    std::thread::scope(|scope| {
        for iteration in 0..24 {
            let context = &context;
            let payload = &payload;
            scope.spawn(move || {
                let run = CommandSpec::catalog_read("finite-output-race", "printf")
                    .public_arg(PublicArg::new("%s"))
                    .public_arg(PublicArg::new(payload))
                    .output_limit(64)
                    .observed_output_limit(64)
                    .run(context)
                    .unwrap();
                let value: serde_json::Value =
                    serde_json::from_slice(&run.to_canonical_json().unwrap()).unwrap();
                assert_eq!(
                    value["termination"]["kind"], "output-limit",
                    "finite iteration {iteration} must not retain the child's zero exit"
                );
                assert_eq!(value["stdout"]["truncated"], true);
                assert_eq!(value["stdout"]["observation_limit_exceeded"], true);
                assert_eq!(value["stdout"]["captured_byte_length_is_lower_bound"], true);
            });
        }
    });
}

#[test]
fn finite_output_at_the_exact_observation_boundary_is_complete_success() {
    let fixture = RepoFixture::new("finite-output-exact-boundary");
    let context = catalog_context(&fixture);
    let payload = "x".repeat(64);

    let run = CommandSpec::catalog_read("finite-output-exact-boundary", "printf")
        .public_arg(PublicArg::new("%s"))
        .public_arg(PublicArg::new(&payload))
        .output_limit(64)
        .observed_output_limit(64)
        .run(&context)
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&run.to_canonical_json().unwrap()).unwrap();

    assert_eq!(value["termination"]["kind"], "exited");
    assert_eq!(value["termination"]["code"], 0);
    assert_eq!(value["stdout"]["captured_byte_length"], 64);
    assert_eq!(value["stdout"]["truncated"], false);
    assert_eq!(value["stdout"]["observation_limit_exceeded"], false);
    assert_eq!(
        value["stdout"]["captured_byte_length_is_lower_bound"],
        false
    );
}

#[test]
fn finite_stdout_and_stderr_share_one_observation_budget() {
    let fixture = RepoFixture::new("finite-combined-output");
    let context = catalog_context(&fixture);
    let format = format!("{}%", "x".repeat(48));

    std::thread::scope(|scope| {
        for iteration in 0..16 {
            let context = &context;
            let format = &format;
            scope.spawn(move || {
                let run = CommandSpec::catalog_read("finite-combined-output", "printf")
                    .public_arg(PublicArg::new(format))
                    .output_limit(64)
                    .observed_output_limit(64)
                    .run(context)
                    .unwrap();
                let value: serde_json::Value =
                    serde_json::from_slice(&run.to_canonical_json().unwrap()).unwrap();
                assert_eq!(value["termination"]["kind"], "output-limit");
                assert!(
                    value["stdout"]["truncated"] == true || value["stderr"]["truncated"] == true,
                    "iteration {iteration} must disclose the combined-budget loser"
                );
                assert!(
                    value["stdout"]["observation_limit_exceeded"] == true
                        || value["stderr"]["observation_limit_exceeded"] == true
                );
            });
        }
    });
}
