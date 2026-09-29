use ultragoal::capture::{ArtifactRef, ArtifactResolver, CapturedRun, CommandSpec};
use ultragoal::context::{
    BuildRequest, CandidateIdentity, CapabilitySet, ContextError, EffectClass, LiveContext,
};
use ultragoal::distribution::{
    DiscoveryObservation, DistributionReport, InstallPlan, InstallSnapshot, MarketplaceSnapshot,
    PackageIdentity, PackagePlan, PackageSnapshot, RuntimeObservation, SurfaceIdentity,
};
use ultragoal::evaluation::{
    EvaluationRun, EvaluationSpec, FailureCase, PromotionDecision, TaskAudit,
};
use ultragoal::fixture_scheduler::{
    ExpectedOutcome, FixtureScheduler, FixtureSpec, IsolationLease,
};
use ultragoal::inventory::{AuthorityCatalog, GeneratedSurfaceIndex, InventoryBuilder};
use ultragoal::migration::{
    CompatibilityRoute, MigrationPlan, MigrationPlanProjection, RetirementDecision,
    RetirementTarget, RetirementTargetProjection,
};
use ultragoal::observability::{
    CausalExplanation, EventQuery, EventStore, ExportAdapter, SemanticEvent,
};
use ultragoal::orchestration::{
    EffectReceipt, EffectRequest, EffectSink, FileJournal, OrchestrationError, Orchestrator,
    ResultCommitment, RootWorkspace, WorkGraph, WorkerResultV1,
};
use ultragoal::repository_fit::{
    FitInspection, FitPlan, FitVerification, Mutation, Ownership, RollbackPlan,
};
use ultragoal::routine_work::{AffectedSet, CoverageDimensions, ImpactGraph, ReuseDecision};
use ultragoal::state::{ClaimCeiling, Finding, NextAction, ProductState, Repair};

fn require_public_type<T>() {}

struct PublicWitnessEffectSink;

impl EffectSink for PublicWitnessEffectSink {
    fn apply(&mut self, _: &EffectRequest) -> Result<EffectReceipt, OrchestrationError> {
        Err(OrchestrationError::EffectDenied)
    }
}

#[test]
fn contract_public_api_witness_compiles_outside_the_library_crate() {
    let _: fn(BuildRequest) -> Result<LiveContext, ContextError> = LiveContext::build;
    require_public_type::<EffectClass>();
    require_public_type::<CapabilitySet>();
    require_public_type::<CandidateIdentity>();
    require_public_type::<InventoryBuilder<'static>>();
    require_public_type::<AuthorityCatalog>();
    require_public_type::<GeneratedSurfaceIndex>();
    require_public_type::<SemanticEvent>();
    require_public_type::<EventStore>();
    require_public_type::<EventQuery>();
    require_public_type::<CausalExplanation>();
    require_public_type::<&'static dyn ExportAdapter>();
    require_public_type::<PackageSnapshot>();
    require_public_type::<InstallSnapshot>();
    require_public_type::<MarketplaceSnapshot>();
    require_public_type::<DiscoveryObservation>();
    require_public_type::<RuntimeObservation>();
    require_public_type::<DistributionReport>();
    require_public_type::<PackageIdentity>();
    require_public_type::<SurfaceIdentity>();
    require_public_type::<PackagePlan>();
    require_public_type::<InstallPlan>();
    require_public_type::<FixtureSpec>();
    require_public_type::<FixtureScheduler>();
    require_public_type::<IsolationLease>();
    require_public_type::<ExpectedOutcome>();
    require_public_type::<Orchestrator<PublicWitnessEffectSink>>();
    require_public_type::<WorkGraph>();
    require_public_type::<WorkerResultV1>();
    require_public_type::<ResultCommitment>();
    require_public_type::<FileJournal>();
    require_public_type::<RootWorkspace>();
    require_public_type::<FitInspection>();
    require_public_type::<FitPlan>();
    require_public_type::<Mutation>();
    require_public_type::<Ownership>();
    require_public_type::<RollbackPlan>();
    require_public_type::<FitVerification>();
    require_public_type::<CommandSpec>();
    require_public_type::<CapturedRun>();
    require_public_type::<ArtifactRef>();
    require_public_type::<&'static dyn ArtifactResolver>();
    require_public_type::<ImpactGraph>();
    require_public_type::<AffectedSet>();
    require_public_type::<ReuseDecision>();
    require_public_type::<CoverageDimensions>();
    require_public_type::<Finding>();
    require_public_type::<Repair>();
    require_public_type::<ProductState>();
    require_public_type::<NextAction>();
    require_public_type::<ClaimCeiling>();
    require_public_type::<EvaluationSpec>();
    require_public_type::<TaskAudit>();
    require_public_type::<EvaluationRun>();
    require_public_type::<FailureCase>();
    require_public_type::<PromotionDecision>();
    require_public_type::<MigrationPlan>();
    require_public_type::<MigrationPlanProjection>();
    require_public_type::<CompatibilityRoute>();
    require_public_type::<RetirementTarget>();
    require_public_type::<RetirementTargetProjection>();
    require_public_type::<RetirementDecision>();
}
