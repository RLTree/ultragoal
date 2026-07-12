#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AgentRole {
    pub(crate) name: &'static str,
    pub(crate) manifest_path: &'static str,
}

// Compiled projection only. Inventory rejects this table unless it is exactly
// reconciled with the adopted PRODUCT_SURFACE_INVENTORY role topology.
pub(crate) const CANONICAL_AGENT_ROLES: [AgentRole; 6] = [
    AgentRole {
        name: "claim-falsifier",
        manifest_path: ".codex/agents/claim-falsifier.toml",
    },
    AgentRole {
        name: "orchestration-recovery-reviewer",
        manifest_path: ".codex/agents/orchestration-recovery-reviewer.toml",
    },
    AgentRole {
        name: "product-journey-reviewer",
        manifest_path: ".codex/agents/product-journey-reviewer.toml",
    },
    AgentRole {
        name: "repo-recon",
        manifest_path: ".codex/agents/repo-recon.toml",
    },
    AgentRole {
        name: "research-verifier",
        manifest_path: ".codex/agents/research-verifier.toml",
    },
    AgentRole {
        name: "security-reviewer",
        manifest_path: ".codex/agents/security-reviewer.toml",
    },
];

pub(crate) fn by_name(name: &str) -> Option<AgentRole> {
    CANONICAL_AGENT_ROLES
        .iter()
        .copied()
        .find(|role| role.name == name)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    #[test]
    fn canonical_roles_are_unique_and_path_bound_without_runtime_assumptions() {
        let roles = super::CANONICAL_AGENT_ROLES;
        assert_eq!(
            roles
                .iter()
                .map(|role| role.name)
                .collect::<BTreeSet<_>>()
                .len(),
            roles.len()
        );
        for role in roles {
            assert_eq!(
                role.manifest_path,
                format!(".codex/agents/{}.toml", role.name)
            );
            assert_eq!(super::by_name(role.name), Some(role));
        }
    }
}
