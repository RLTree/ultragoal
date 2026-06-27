pub(crate) struct PersonaSpec {
    pub(crate) persona: &'static str,
    pub(crate) agent_type: &'static str,
    pub(crate) prompt_path: &'static str,
    pub(crate) custom_path: &'static str,
    pub(crate) focus_path: &'static str,
}

pub(crate) const PERSONAS: &[PersonaSpec] = &[
    PersonaSpec {
        persona: "contract_claim_falsifier",
        agent_type: "harness_contract_claim_falsifier",
        prompt_path: "agents/contract-claim-falsifier.md",
        custom_path: "custom-agents/harness-contract-claim-falsifier.toml",
        focus_path: "docs/hypercritical-review-law.md",
    },
    PersonaSpec {
        persona: "orchestration_recovery_falsifier",
        agent_type: "harness_orchestration_recovery_falsifier",
        prompt_path: "agents/orchestration-recovery-falsifier.md",
        custom_path: "custom-agents/harness-orchestration-recovery-falsifier.toml",
        focus_path: "validator/src/review/round/registry.rs",
    },
    PersonaSpec {
        persona: "security_trust_boundary_falsifier",
        agent_type: "harness_security_trust_boundary_falsifier",
        prompt_path: "agents/security-trust-boundary-falsifier.md",
        custom_path: "custom-agents/harness-security-trust-boundary-falsifier.toml",
        focus_path: "validator/src/schema_catalog/mod.rs",
    },
    PersonaSpec {
        persona: "product_simplicity_falsifier",
        agent_type: "harness_product_simplicity_falsifier",
        prompt_path: "agents/product-simplicity-falsifier.md",
        custom_path: "custom-agents/harness-product-simplicity-falsifier.toml",
        focus_path: "docs/plugin-resource-map.md",
    },
];

pub(crate) fn persona_spec(persona: &str) -> Option<&'static PersonaSpec> {
    PERSONAS.iter().find(|spec| spec.persona == persona)
}

pub(crate) fn expected_model(round_phase: &str) -> &'static str {
    let _ = round_phase;
    "gpt-5.5"
}

pub(crate) fn expected_verdict(round_phase: &str) -> &'static str {
    let _ = round_phase;
    "SIGN_OFF"
}

pub(crate) fn full_anchor_required(round_phase: &str, anchor_policy: &str) -> bool {
    round_phase == "sign_off" && anchor_policy == "validator_review_target_archive"
}
