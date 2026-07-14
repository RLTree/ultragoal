use super::artifact;
use super::inputs::{
    ArgumentInput, ArtifactExpectation, EnvironmentInput, PublicArg, PublicArtifact, PublicEnv,
    SecretArg, SecretArtifact, SecretEnv,
};
use super::path_policy::{os_bytes, validate_public_path, validate_relative};
use super::run::{self, CapturedRun};
use crate::context::{EffectClass, LiveContext};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[path = "command_specification.rs"]
mod command_specification;
#[path = "specification_limits.rs"]
mod specification_limits;

pub(crate) use command_specification::*;
pub use specification_limits::CommandSpec;
pub(crate) use specification_limits::{
    CATALOG_BINDING_UNAVAILABLE, CatalogBinding, CatalogPermit, DEFAULT_OBSERVED_OUTPUT_LIMIT,
    DEFAULT_OUTPUT_LIMIT, MAX_ARGUMENTS, MAX_ARTIFACTS, MAX_ENVIRONMENT_ENTRIES,
    MAX_OBSERVED_OUTPUT_LIMIT, MAX_OUTPUT_LIMIT, MAX_PATH_BYTES, MAX_TIMEOUT,
    validate_path_byte_bound,
};
