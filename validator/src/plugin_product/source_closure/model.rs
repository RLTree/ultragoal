use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildInputKind {
    CargoLock,
    CargoManifest,
    DepInfo,
    RustSource,
    RuntimeAuthority,
    VerifierInput,
    DynamicInput,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredBuildInput {
    pub path: String,
    pub kind: BuildInputKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildClosurePolicy {
    pub schema_version: String,
    pub required_inputs: Vec<RequiredBuildInput>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildClosureRow {
    pub path: String,
    pub kind: BuildInputKind,
    pub sha256: String,
    pub byte_length: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildClosureV1 {
    pub schema_version: String,
    pub rows: Vec<BuildClosureRow>,
    pub aggregate_sha256: String,
    pub final_session_revalidated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClosureError {
    DigestMismatch,
    DuplicateOrAlias,
    FinalSessionDrift,
    InvalidDepInfo,
    InvalidDigest,
    InvalidPath,
    Missing,
    OutsideRoot,
    ResourceLimit,
    SpecialFile,
    UnknownRow,
    Unreadable,
}
