const ENVELOPE_PATH: &str =
    "docs/ultragoal-successor-live/work-packages/CANONICAL-PLUGIN-DEPENDENCY-CLOSURE-018.json";
const RESULT_PATH: &str =
    "docs/ultragoal-successor-live/worker-results/CANONICAL-PLUGIN-DEPENDENCY-CLOSURE-018.json";
const ENVELOPE_SHA256: &str =
    "sha256:9cd652e0eb96f1a0d51a77385728c46a87487e5d8b0b65f7062e9d1b16035d7e";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootIssuedEnvelope {
    schema_version: String,
    issued_from_live_context: IssuedContext,
    work_package: WorkPackage,
    lease: LeaseSpec,
    protected_root_inputs: Vec<ProtectedRootInput>,
    root_validation_obligations: Vec<String>,
    no_claim_statement: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IssuedContext {
    context_id: String,
    candidate_id: String,
    head_commit: String,
    head_tree: String,
    branch: String,
    dirty: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtectedRootInput {
    path: String,
    #[serde(default)]
    precondition: Option<String>,
    sha256: String,
    #[serde(default)]
    byte_length: Option<u64>,
    #[serde(default)]
    line_count: Option<u64>,
    worker_access: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateClosure {
    schema_version: String,
    artifact_set_formula: String,
    worker_result_self_exclusion: String,
    members: Vec<Member>,
    executable_read_operations: Vec<Operation>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Member {
    path: String,
    authority: Authority,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
enum Authority {
    WorkerOwned,
    RootRead,
    ProtectedRootMetadata,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    id: String,
    args: Vec<String>,
    source_marker: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DescriptorBinding {
    name: String,
    path: String,
    sha256: String,
    byte_length: u64,
    line_count: u64,
}

#[derive(Debug, Eq, PartialEq)]
enum ClosureError {
    Conflict,
    Missing,
    RootMetadataMismatch,
    Unknown,
}

fn envelope() -> RootIssuedEnvelope {
    serde_json::from_str(&read(ENVELOPE_PATH))
        .unwrap_or_else(|error| panic!("typed root-issued envelope invalid: {error}"))
}

fn root_request() -> Value {
    serde_json::from_str(&read("fixtures/plugin-product/root-wiring-request.json"))
        .unwrap_or_else(|error| panic!("root wiring request invalid: {error}"))
}

fn closure() -> CandidateClosure {
    serde_json::from_value(root_request()["candidate_closure"].clone())
        .unwrap_or_else(|error| panic!("typed candidate closure invalid: {error}"))
}
