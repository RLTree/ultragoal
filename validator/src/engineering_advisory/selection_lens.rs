use super::AdvisoryLens;

impl AdvisoryLens {
    pub fn skill_name(self) -> &'static str {
        match self {
            Self::AgenticEngineering => "agentic-engineering",
            Self::CodexTaskContract => "codex-task-contract",
            Self::ContextRepositoryEngineering => "context-repository-engineering",
            Self::AgentProductDiscovery => "agent-product-discovery",
            Self::ConceptFeasibilityValidation => "concept-feasibility-validation",
            Self::AuthenticUseEngineering => "authentic-use-engineering",
            Self::RequirementsSystemsEngineering => "requirements-systems-engineering",
            Self::AgenticProductLifecycle => "agentic-product-lifecycle",
            Self::ArchitectureDeliveryPlanning => "architecture-delivery-planning",
            Self::AgentFirstSoftwareEngineering => "agent-first-software-engineering",
            Self::SoftwareConstructionQuality => "software-construction-quality",
            Self::HarnessEngineering => "harness-engineering",
            Self::LoopEngineering => "loop-engineering",
            Self::GraphWorkflowEngineering => "graph-workflow-engineering",
            Self::MultiAgentEngineering => "multi-agent-engineering",
            Self::VerificationStrategyEngineering => "verification-strategy-engineering",
            Self::AgentEvalsObservability => "agent-evals-observability",
            Self::AgentSecurityGovernance => "agent-security-governance",
            Self::RustAgenticArchitecture => "rust-agentic-architecture",
            Self::RustAgentRuntime => "rust-agent-runtime",
            Self::RustAgentDurability => "rust-agent-durability",
            Self::RustAgentProtocols => "rust-agent-protocols",
            Self::RustAgentVerification => "rust-agent-verification",
            Self::RustAgentObservability => "rust-agent-observability",
            Self::ProductFitnessEngineering => "product-fitness-engineering",
            Self::ContinuousProductExperimentation => "continuous-product-experimentation",
            Self::SecureDeliveryRelease => "secure-delivery-release",
            Self::ProductionReadinessSre => "production-readiness-sre",
            Self::MaintenanceRetirementEngineering => "maintenance-retirement-engineering",
            Self::EngineeringLearningLoop => "engineering-learning-loop",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::AgenticEngineering,
            Self::CodexTaskContract,
            Self::ContextRepositoryEngineering,
            Self::AgentProductDiscovery,
            Self::ConceptFeasibilityValidation,
            Self::AuthenticUseEngineering,
            Self::RequirementsSystemsEngineering,
            Self::AgenticProductLifecycle,
            Self::ArchitectureDeliveryPlanning,
            Self::AgentFirstSoftwareEngineering,
            Self::SoftwareConstructionQuality,
            Self::HarnessEngineering,
            Self::LoopEngineering,
            Self::GraphWorkflowEngineering,
            Self::MultiAgentEngineering,
            Self::VerificationStrategyEngineering,
            Self::AgentEvalsObservability,
            Self::AgentSecurityGovernance,
            Self::RustAgenticArchitecture,
            Self::RustAgentRuntime,
            Self::RustAgentDurability,
            Self::RustAgentProtocols,
            Self::RustAgentVerification,
            Self::RustAgentObservability,
            Self::ProductFitnessEngineering,
            Self::ContinuousProductExperimentation,
            Self::SecureDeliveryRelease,
            Self::ProductionReadinessSre,
            Self::MaintenanceRetirementEngineering,
            Self::EngineeringLearningLoop,
        ]
    }

    pub(crate) fn rank(self) -> u8 {
        match self {
            Self::AgentSecurityGovernance => 0,
            Self::RustAgentDurability | Self::RustAgentProtocols => 1,
            Self::VerificationStrategyEngineering | Self::RustAgentVerification => 2,
            Self::CodexTaskContract | Self::ContextRepositoryEngineering => 3,
            Self::LoopEngineering | Self::HarnessEngineering => 4,
            Self::ProductFitnessEngineering | Self::AuthenticUseEngineering => 5,
            _ => 6,
        }
    }

    pub(crate) fn owner_family(self) -> &'static str {
        match self {
            Self::AgenticEngineering
            | Self::CodexTaskContract
            | Self::ContextRepositoryEngineering => "framing",
            Self::AgentProductDiscovery
            | Self::ConceptFeasibilityValidation
            | Self::AuthenticUseEngineering
            | Self::RequirementsSystemsEngineering
            | Self::AgenticProductLifecycle
            | Self::ProductFitnessEngineering
            | Self::ContinuousProductExperimentation
            | Self::SecureDeliveryRelease
            | Self::ProductionReadinessSre
            | Self::MaintenanceRetirementEngineering => "product_lifecycle",
            Self::ArchitectureDeliveryPlanning
            | Self::AgentFirstSoftwareEngineering
            | Self::SoftwareConstructionQuality => "construction",
            Self::HarnessEngineering
            | Self::LoopEngineering
            | Self::GraphWorkflowEngineering
            | Self::MultiAgentEngineering => "orchestration",
            Self::VerificationStrategyEngineering
            | Self::AgentEvalsObservability
            | Self::AgentSecurityGovernance
            | Self::EngineeringLearningLoop => "assurance",
            Self::RustAgenticArchitecture
            | Self::RustAgentRuntime
            | Self::RustAgentDurability
            | Self::RustAgentProtocols
            | Self::RustAgentVerification
            | Self::RustAgentObservability => "rust",
        }
    }
}
