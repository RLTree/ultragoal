use serde::Serialize;

pub(crate) const RUST_RECEIPT_SCHEMA: &str = "harness-ultragoal.rust-devx-receipt.v1";
pub(crate) const RUST_POLICY_VERSION: &str = "2026-06-26.gate-91.v1";

pub(crate) const RUST_COMMANDS: &[&str] = &[
    "ultragoal rust toolchain verify",
    "ultragoal rust fast",
    "ultragoal rust standard",
    "ultragoal rust release",
    "ultragoal rust clean-proof",
    "ultragoal rust watch",
    "ultragoal rust memory prove",
    "ultragoal rust dependency audit",
    "ultragoal rust coverage prove --exact",
    "ultragoal rust workspace topology check",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RustOperation {
    ToolchainVerify,
    Fast,
    Standard,
    Release,
    CleanProof,
    Watch,
    MemoryProve,
    DependencyAudit,
    CoverageProve,
    WorkspaceTopology,
}

impl RustOperation {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::ToolchainVerify => "toolchain_verify",
            Self::Fast => "fast",
            Self::Standard => "standard",
            Self::Release => "release",
            Self::CleanProof => "clean_proof",
            Self::Watch => "watch",
            Self::MemoryProve => "memory_prove",
            Self::DependencyAudit => "dependency_audit",
            Self::CoverageProve => "coverage_prove",
            Self::WorkspaceTopology => "workspace_topology",
        }
    }

    pub(crate) fn law_id(self) -> &'static str {
        match self {
            Self::ToolchainVerify => "rust-toolchain-substrate-authority",
            Self::Fast | Self::Standard | Self::Release | Self::Watch | Self::CoverageProve => {
                "rust-command-loop-authority"
            }
            Self::CleanProof => "rust-cache-no-cache-honesty",
            Self::MemoryProve => "rust-memory-resource-discipline",
            Self::DependencyAudit | Self::WorkspaceTopology => {
                "rust-developer-experience-authority"
            }
        }
    }

    pub(crate) fn proof_surface(self) -> &'static str {
        match self {
            Self::ToolchainVerify => "toolchain_substrate",
            Self::Fast => "fast_feedback",
            Self::Standard => "standard_repair_proof",
            Self::Release => "release_loop_observation",
            Self::CleanProof => "clean_checkout_no_hidden_cache",
            Self::Watch => "watch_observation",
            Self::MemoryProve => "memory_resource_discipline",
            Self::DependencyAudit => "dependency_supply_chain",
            Self::CoverageProve => "exact_coverage",
            Self::WorkspaceTopology => "workspace_module_topology",
        }
    }
}
