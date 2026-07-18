pub(crate) const READER_PROOF_PATH: &str =
    "docs/ultragoal-successor-live/worker-results/LEASE-N02-AGENT-READERS-002.json";
pub(crate) const READER_PROOF_SHA256: &str =
    "47a5690209d11d3bbea3bc3d2b2a14c7de3404e09364e9ad48ebe0df85f40d99";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TargetState {
    ActiveAgent,
    AdoptedProductDefinition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AgentRouteSpec {
    pub route_id: &'static str,
    pub legacy_path: &'static str,
    pub legacy_sha256: &'static str,
    pub canonical_target: &'static str,
    pub target_path: &'static str,
    pub target_sha256: &'static str,
    pub target_state: TargetState,
}

impl AgentRouteSpec {
    pub fn stable_id(&self) -> String {
        format!("LEGACY-AGENT:{}", self.legacy_path)
    }

    pub fn proof_refs_match(&self, proof_refs: &[String]) -> bool {
        proof_refs.iter().map(String::as_str).eq([
            self.legacy_path,
            self.target_path,
            READER_PROOF_PATH,
        ])
    }
}

const PS_FIT_FRESH_PATH: &str = "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/PRODUCT_SURFACE_INVENTORY.json#/product-surface-definition/PS-FIT-FRESH";
const PS_FIT_RETROFIT_PATH: &str = "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/PRODUCT_SURFACE_INVENTORY.json#/product-surface-definition/PS-FIT-RETROFIT";

const ROUTES: [AgentRouteSpec; 14] = [
    AgentRouteSpec {
        route_id: "agent-contract-claim-falsifier-to-claim-falsifier",
        legacy_path: "agents/contract-claim-falsifier.md",
        legacy_sha256: "4a4f23123620793e816fc29aaddac076c07867c9055af53a28838399f614182f",
        canonical_target: "AGENT:claim-falsifier",
        target_path: ".codex/agents/claim-falsifier.toml",
        target_sha256: "6ecb0324f07793dd2c8581cd82b83ae6086095fea5d4715c550076211c324e6f",
        target_state: TargetState::ActiveAgent,
    },
    AgentRouteSpec {
        route_id: "agent-harness-contract-claim-falsifier-to-claim-falsifier",
        legacy_path: "custom-agents/harness-contract-claim-falsifier.toml",
        legacy_sha256: "637d4e212900a16933180c93646468aa878914dc36111050799e10043de25a6d",
        canonical_target: "AGENT:claim-falsifier",
        target_path: ".codex/agents/claim-falsifier.toml",
        target_sha256: "6ecb0324f07793dd2c8581cd82b83ae6086095fea5d4715c550076211c324e6f",
        target_state: TargetState::ActiveAgent,
    },
    AgentRouteSpec {
        route_id: "agent-orchestration-recovery-falsifier-to-orchestration-recovery-reviewer",
        legacy_path: "agents/orchestration-recovery-falsifier.md",
        legacy_sha256: "bcf785b28af425a4d7c43f8a18cd2919a905bf8b1b5a62543b8ae660f5618593",
        canonical_target: "AGENT:orchestration-recovery-reviewer",
        target_path: ".codex/agents/orchestration-recovery-reviewer.toml",
        target_sha256: "da39757a9d58499821f6c11863d1f6bb1d64e015826837ead3962de9c0e6fb75",
        target_state: TargetState::ActiveAgent,
    },
    AgentRouteSpec {
        route_id: "agent-harness-orchestration-recovery-falsifier-to-orchestration-recovery-reviewer",
        legacy_path: "custom-agents/harness-orchestration-recovery-falsifier.toml",
        legacy_sha256: "5666c376c73af8b5f83cfcd1d9afbb1976efe69bc36fc3ad26f4543b8fad3003",
        canonical_target: "AGENT:orchestration-recovery-reviewer",
        target_path: ".codex/agents/orchestration-recovery-reviewer.toml",
        target_sha256: "da39757a9d58499821f6c11863d1f6bb1d64e015826837ead3962de9c0e6fb75",
        target_state: TargetState::ActiveAgent,
    },
    AgentRouteSpec {
        route_id: "agent-security-trust-boundary-falsifier-to-security-reviewer",
        legacy_path: "agents/security-trust-boundary-falsifier.md",
        legacy_sha256: "682d2435dcf3a049ae1551cba6feb87180c743bd9897c297a33411eadb90b9e1",
        canonical_target: "AGENT:security-reviewer",
        target_path: ".codex/agents/security-reviewer.toml",
        target_sha256: "20efde18349120ad57c938a678ea7e6fa076dd454201ec2ec6e3d9d6fe670b65",
        target_state: TargetState::ActiveAgent,
    },
    AgentRouteSpec {
        route_id: "agent-harness-security-trust-boundary-falsifier-to-security-reviewer",
        legacy_path: "custom-agents/harness-security-trust-boundary-falsifier.toml",
        legacy_sha256: "13125d9551b085ef9fbc6b21434ef0ced7a1717df6b3a51ecc3f2ddfcca602af",
        canonical_target: "AGENT:security-reviewer",
        target_path: ".codex/agents/security-reviewer.toml",
        target_sha256: "20efde18349120ad57c938a678ea7e6fa076dd454201ec2ec6e3d9d6fe670b65",
        target_state: TargetState::ActiveAgent,
    },
    AgentRouteSpec {
        route_id: "agent-product-simplicity-falsifier-to-product-journey-reviewer",
        legacy_path: "agents/product-simplicity-falsifier.md",
        legacy_sha256: "9232988103fcf7e99c42b610bed9210dc1d4f68d7690e9e3be5070cc04ac5d84",
        canonical_target: "AGENT:product-journey-reviewer",
        target_path: ".codex/agents/product-journey-reviewer.toml",
        target_sha256: "9d5f4955afbf33a0965676ad00474aabe4843b3345e901fa0e518b0c9daf6a32",
        target_state: TargetState::ActiveAgent,
    },
    AgentRouteSpec {
        route_id: "agent-harness-product-simplicity-falsifier-to-product-journey-reviewer",
        legacy_path: "custom-agents/harness-product-simplicity-falsifier.toml",
        legacy_sha256: "cf27b760170ec93113c4fd6f648f3c04f9e4b0fa7e86715e888fbeb9a20897b3",
        canonical_target: "AGENT:product-journey-reviewer",
        target_path: ".codex/agents/product-journey-reviewer.toml",
        target_sha256: "9d5f4955afbf33a0965676ad00474aabe4843b3345e901fa0e518b0c9daf6a32",
        target_state: TargetState::ActiveAgent,
    },
    AgentRouteSpec {
        route_id: "agent-material-review-scope-gatekeeper-to-product-journey-reviewer",
        legacy_path: "agents/material-review-scope-gatekeeper.md",
        legacy_sha256: "f5ca10859a19854562a57ffc810b076d692bcf705b490225a1a22733ca977622",
        canonical_target: "AGENT:product-journey-reviewer",
        target_path: ".codex/agents/product-journey-reviewer.toml",
        target_sha256: "9d5f4955afbf33a0965676ad00474aabe4843b3345e901fa0e518b0c9daf6a32",
        target_state: TargetState::ActiveAgent,
    },
    AgentRouteSpec {
        route_id: "agent-harness-material-review-scope-gatekeeper-to-product-journey-reviewer",
        legacy_path: "custom-agents/harness-material-review-scope-gatekeeper.toml",
        legacy_sha256: "bf9e7da1d5995bb38c3dd586e022064508afdc36172d220d1ba49ee89ec57133",
        canonical_target: "AGENT:product-journey-reviewer",
        target_path: ".codex/agents/product-journey-reviewer.toml",
        target_sha256: "9d5f4955afbf33a0965676ad00474aabe4843b3345e901fa0e518b0c9daf6a32",
        target_state: TargetState::ActiveAgent,
    },
    AgentRouteSpec {
        route_id: "agent-plugin-scout-to-repo-recon",
        legacy_path: "agents/plugin-scout.md",
        legacy_sha256: "43d918365258daa6b34c2f92d575e88eabd95aa32cd5af52021ef9008936a104",
        canonical_target: "AGENT:repo-recon",
        target_path: ".codex/agents/repo-recon.toml",
        target_sha256: "4272e86928d359a867da83491497ee163a1f66c9cf99683df69bec7d55b91f9c",
        target_state: TargetState::ActiveAgent,
    },
    AgentRouteSpec {
        route_id: "agent-standards-extractor-to-research-verifier",
        legacy_path: "agents/standards-extractor.md",
        legacy_sha256: "5c8e4b49356fb1e513e85fb4b0e95e985aec84ead7ae1a6faf6358c025004789",
        canonical_target: "AGENT:research-verifier",
        target_path: ".codex/agents/research-verifier.toml",
        target_sha256: "47ee7d30b1052e1c990bc65f9dcaefb77f341b80d1dcbe0fd2d024daeb5901ee",
        target_state: TargetState::ActiveAgent,
    },
    AgentRouteSpec {
        route_id: "agent-harness-repo-initializer-to-fit-fresh",
        legacy_path: "custom-agents/harness-repo-initializer.toml",
        legacy_sha256: "7c237e2b8d9b65329144ba93407a8d8db47b2a80e660b176ad3b43a3ff19d4bb",
        canonical_target: "PS-FIT-FRESH",
        target_path: PS_FIT_FRESH_PATH,
        target_sha256: "69c8e65cdafc70c09f87368657f7956a778397151e79682752c1045a8a80efdd",
        target_state: TargetState::AdoptedProductDefinition,
    },
    AgentRouteSpec {
        route_id: "agent-harness-retrofit-planner-to-fit-retrofit",
        legacy_path: "custom-agents/harness-retrofit-planner.toml",
        legacy_sha256: "f16db61ef153feb99b03d97a095550ef2bec518bf355e0abeca63f8b8ccf8b49",
        canonical_target: "PS-FIT-RETROFIT",
        target_path: PS_FIT_RETROFIT_PATH,
        target_sha256: "dfba84655a260176ee8b9d3c3f3f3bd954b98b406fd8f8f815796ce0861c023e",
        target_state: TargetState::AdoptedProductDefinition,
    },
];

pub(crate) fn by_agent_route_id(route_id: &str) -> Option<&'static AgentRouteSpec> {
    ROUTES.iter().find(|route| route.route_id == route_id)
}
