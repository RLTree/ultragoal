use crate::audit::contract::Failure;
use std::collections::BTreeSet;

const CUSTOM_AGENT_PATHS: &[&str] = &[
    "custom-agents/harness-contract-adversary.toml",
    "custom-agents/harness-orchestration-adversary.toml",
    "custom-agents/harness-production-experience-gatekeeper.toml",
    "custom-agents/harness-security-trust-boundary-adversary.toml",
    "custom-agents/harness-simplicity-auditor.toml",
    "custom-agents/harness-verification-gatekeeper.toml",
];

const PROMPT_PACKET_PATHS: &[&str] = &[
    "fixtures/review-round/prompt-packets/contract_adversary.md",
    "fixtures/review-round/prompt-packets/orchestration_adversary.md",
    "fixtures/review-round/prompt-packets/production_experience_gatekeeper.md",
    "fixtures/review-round/prompt-packets/security_trust_boundary_adversary.md",
    "fixtures/review-round/prompt-packets/simplicity_auditor.md",
    "fixtures/review-round/prompt-packets/verification_gatekeeper.md",
];

pub(crate) fn check_packaged_paths(plugin_paths: &[String], out: &mut Vec<Failure>) {
    let paths = plugin_paths
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    reject_paths(
        &paths,
        CUSTOM_AGENT_PATHS,
        "retired_reviewer_custom_agent_packaged",
        out,
    );
    reject_paths(
        &paths,
        PROMPT_PACKET_PATHS,
        "retired_reviewer_prompt_packet_packaged",
        out,
    );
}

fn reject_paths(
    packaged: &BTreeSet<&str>,
    retired: &[&'static str],
    code: &'static str,
    out: &mut Vec<Failure>,
) {
    for path in retired {
        if packaged.contains(path) {
            out.push(Failure::new("plugin-inventory-closure", code, *path));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_reviewer_custom_agent_paths_are_rejected() {
        let paths = vec!["custom-agents/harness-contract-adversary.toml".to_string()];
        let mut out = Vec::new();

        check_packaged_paths(&paths, &mut out);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].check_id, "plugin-inventory-closure");
        assert_eq!(out[0].error, "retired_reviewer_custom_agent_packaged");
    }

    #[test]
    fn retired_reviewer_prompt_packets_are_rejected() {
        let paths =
            vec!["fixtures/review-round/prompt-packets/verification_gatekeeper.md".to_string()];
        let mut out = Vec::new();

        check_packaged_paths(&paths, &mut out);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].check_id, "plugin-inventory-closure");
        assert_eq!(out[0].error, "retired_reviewer_prompt_packet_packaged");
    }
}
