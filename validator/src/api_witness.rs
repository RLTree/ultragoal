use crate::capture::{ArtifactRef, ArtifactResolver, CapturedRun, CommandSpec};
use crate::context::{
    BuildRequest, CandidateIdentity, CapabilitySet, ContextError, EffectClass, LiveContext,
};
use crate::distribution::{
    DiscoveryObservation, DistributionReport, InstallPlan, InstallSnapshot, MarketplaceSnapshot,
    PackageIdentity, PackagePlan, PackageSnapshot, RuntimeObservation, SurfaceIdentity,
};
use crate::inventory::{AuthorityCatalog, GeneratedSurfaceIndex, InventoryBuilder};
use crate::observability::{
    CausalExplanation, EventQuery, EventStore, ExportAdapter, SemanticEvent,
};
use crate::orchestration::{
    EffectReceipt, EffectRequest, EffectSink, FileJournal, OrchestrationError, Orchestrator,
    ResultCommitment, RootWorkspace, WorkGraph, WorkerResultV1,
};
use crate::repository_fit::{
    FitInspection, FitPlan, FitVerification, Mutation, Ownership, RollbackPlan,
};
use crate::routine_work::{AffectedSet, CoverageDimensions, ImpactGraph, ReuseDecision};
use crate::state::{ClaimCeiling, Finding, NextAction, ProductState, Repair};

fn require_type<T>() {}

struct WitnessEffectSink;

impl EffectSink for WitnessEffectSink {
    fn apply(&mut self, _: &EffectRequest) -> Result<EffectReceipt, OrchestrationError> {
        Err(OrchestrationError::EffectDenied)
    }
}

pub(crate) fn implemented_public_apis() -> &'static [&'static str] {
    let _: fn(BuildRequest) -> Result<LiveContext, ContextError> = LiveContext::build;
    require_type::<EffectClass>();
    require_type::<CapabilitySet>();
    require_type::<CandidateIdentity>();
    require_type::<InventoryBuilder<'static>>();
    require_type::<AuthorityCatalog>();
    require_type::<GeneratedSurfaceIndex>();
    require_type::<SemanticEvent>();
    require_type::<EventStore>();
    require_type::<EventQuery>();
    require_type::<CausalExplanation>();
    require_type::<&'static dyn ExportAdapter>();
    require_type::<PackageSnapshot>();
    require_type::<InstallSnapshot>();
    require_type::<MarketplaceSnapshot>();
    require_type::<DiscoveryObservation>();
    require_type::<RuntimeObservation>();
    require_type::<DistributionReport>();
    require_type::<PackageIdentity>();
    require_type::<SurfaceIdentity>();
    require_type::<PackagePlan>();
    require_type::<InstallPlan>();
    require_type::<Orchestrator<WitnessEffectSink>>();
    require_type::<WorkGraph>();
    require_type::<WorkerResultV1>();
    require_type::<ResultCommitment>();
    require_type::<FileJournal>();
    require_type::<RootWorkspace>();
    require_type::<FitInspection>();
    require_type::<FitPlan>();
    require_type::<Mutation>();
    require_type::<Ownership>();
    require_type::<RollbackPlan>();
    require_type::<FitVerification>();
    require_type::<CommandSpec>();
    require_type::<CapturedRun>();
    require_type::<ArtifactRef>();
    require_type::<&'static dyn ArtifactResolver>();
    require_type::<ImpactGraph>();
    require_type::<AffectedSet>();
    require_type::<ReuseDecision>();
    require_type::<CoverageDimensions>();
    require_type::<Finding>();
    require_type::<Repair>();
    require_type::<ProductState>();
    require_type::<NextAction>();
    require_type::<ClaimCeiling>();
    // This list is an exact witness for adopted custom-tool API identifiers,
    // not a catalog of every compile-visible library type checked above.
    // Additional distribution and orchestration types below remain visibility
    // witnesses only and do not create unadopted custom-tool API identifiers.
    &[
        "LiveContext::build",
        "EffectClass",
        "CapabilitySet",
        "CandidateIdentity",
        "InventoryBuilder",
        "AuthorityCatalog",
        "GeneratedSurfaceIndex",
        "SemanticEvent",
        "EventStore",
        "EventQuery",
        "CausalExplanation",
        "ExportAdapter",
        "PackageSnapshot",
        "InstallSnapshot",
        "MarketplaceSnapshot",
        "DiscoveryObservation",
        "RuntimeObservation",
        "FitInspection",
        "FitPlan",
        "Mutation",
        "Ownership",
        "RollbackPlan",
        "FitVerification",
        "CommandSpec",
        "CapturedRun",
        "ArtifactRef",
        "ArtifactResolver",
        "ImpactGraph",
        "AffectedSet",
        "ReuseDecision",
        "CoverageDimensions",
        "Finding",
        "Repair",
        "ProductState",
        "NextAction",
        "ClaimCeiling",
    ]
}
