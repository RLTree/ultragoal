use std::collections::BTreeSet;

pub(super) const ORDER: [&str; 4] = [
    "agentic-engineering",
    "agentic-engineering-lifecycle",
    "agentic-engineering-rust",
    "agentic-engineering-systems",
];

pub(super) const BASE_SKILLS: [&str; 8] = [
    "agent-evals-observability",
    "agent-security-governance",
    "agentic-product-lifecycle",
    "authentic-use-engineering",
    "codex-task-contract",
    "engineering-learning-loop",
    "product-fitness-engineering",
    "verification-strategy-engineering",
];

pub(super) const LIFECYCLE_SKILLS: [&str; 10] = [
    "agentic-engineering",
    "agent-product-discovery",
    "architecture-delivery-planning",
    "concept-feasibility-validation",
    "continuous-product-experimentation",
    "maintenance-retirement-engineering",
    "production-readiness-sre",
    "requirements-systems-engineering",
    "secure-delivery-release",
    "software-construction-quality",
];

pub(super) const RUST_SKILLS: [&str; 7] = [
    "harness-engineering",
    "rust-agent-durability",
    "rust-agent-observability",
    "rust-agent-protocols",
    "rust-agent-runtime",
    "rust-agent-verification",
    "rust-agentic-architecture",
];

pub(super) const SYSTEMS_SKILLS: [&str; 5] = [
    "agent-first-software-engineering",
    "context-repository-engineering",
    "graph-workflow-engineering",
    "loop-engineering",
    "multi-agent-engineering",
];

pub(super) fn expected_skills(name: &str) -> Option<BTreeSet<&'static str>> {
    let skills = match name {
        "agentic-engineering" => &BASE_SKILLS[..],
        "agentic-engineering-lifecycle" => &LIFECYCLE_SKILLS[..],
        "agentic-engineering-rust" => &RUST_SKILLS[..],
        "agentic-engineering-systems" => &SYSTEMS_SKILLS[..],
        _ => return None,
    };
    Some(skills.iter().copied().collect())
}
