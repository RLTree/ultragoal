// Included by the root-owned validator/tests/cli_contract.rs after it provides
// a crate::context adapter re-exporting ultragoal::context.
#[path = "../../../src/cli/capture/mod.rs"]
mod capture;

#[path = "artifact/secret_policy.rs"]
mod artifact_secret_policy;
#[path = "artifact/secret_safety.rs"]
mod artifact_secret_safety;
#[path = "artifact/secrets.rs"]
mod artifact_secrets;
#[path = "artifact/special_files.rs"]
mod artifact_special_files;
mod artifacts;
mod bounds;
mod enforcement;
mod fixture;
mod linked_worktree_enforcement;
mod output_runtime_secret_safety;
mod process;
mod process_catalog_rejection;
mod program_security;
#[path = "secret/artifact_length_oracle.rs"]
mod secret_artifact_length_oracle;
#[path = "secret/capture_fixture.rs"]
mod secret_capture_fixture;
#[path = "secret/length_oracle.rs"]
mod secret_length_oracle;
mod security;
#[path = "transformed/secret/artifacts.rs"]
mod transformed_secret_artifacts;
#[path = "transformed/secret/metadata.rs"]
mod transformed_secret_metadata;
#[path = "transformed/secret/projection.rs"]
mod transformed_secret_projection;
#[path = "transformed/secret/safety.rs"]
mod transformed_secret_safety;
