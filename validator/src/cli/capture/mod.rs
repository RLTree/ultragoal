//! Candidate-bound capture machinery and immutable artifact storage.
//!
//! Production capture accepts no caller-minted execution authority. Shells,
//! wrappers, unprotected programs, writes, network access, and process forking
//! remain unavailable.
//! Every caller-supplied `CommandSpec` fails before context revalidation or
//! descriptor access.
//! Secret values are never inferred.

mod artifact;
mod artifact_model;
mod artifact_safety;
mod descriptor;
#[cfg(test)]
mod descriptor_race_control;
mod environment;
mod filesystem;
mod fixture;
mod identity_codec;
mod inputs;
mod output;
mod path_policy;
mod process;
mod program;
mod run;
mod sandbox;
mod spec;
mod tree_witness_adapter;

pub use artifact_model::{ArtifactDisposition, ArtifactRef, ArtifactResolver, CapturedArtifact};
pub(crate) use fixture::ScheduledFixtureEvaluationBridge;
pub use fixture::execute_scheduled_fixture;
pub use inputs::{PublicArg, PublicArtifact, PublicEnv, SecretArg, SecretArtifact, SecretEnv};
pub use run::CapturedRun;
pub use spec::CommandSpec;

#[cfg(test)]
pub use artifact::{capture_public_for_test, capture_spec_artifacts_for_test};
#[cfg(test)]
pub use artifact_safety::finalize_for_test;
#[cfg(test)]
pub use descriptor::{
    reset_test_descriptor_bytes_read, reset_test_file_open_attempts, test_descriptor_bytes_read,
    test_file_open_attempts,
};
#[cfg(test)]
pub use descriptor_race_control::{set_test_preopen_pause_ms, test_preopen_is_paused};
#[cfg(test)]
pub use filesystem::{set_test_artifact_pause_ms, test_artifact_is_paused};
