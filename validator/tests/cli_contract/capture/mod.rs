// Included by the root-owned validator/tests/cli_contract.rs after it provides
// a crate::context adapter re-exporting ultragoal::context.
#[path = "../../../src/cli/capture/mod.rs"]
mod capture;

mod artifact_secret_policy;
mod artifact_secret_safety;
mod artifact_secret_support;
mod artifact_secrets;
mod artifact_special_files;
mod artifacts;
mod bounds;
mod enforcement;
mod fixture;
mod output_runtime_secret_safety;
mod process;
mod program_security;
mod secret_length_oracle;
mod security;
mod transformed_secret_artifacts;
mod transformed_secret_metadata;
mod transformed_secret_safety;
