pub(super) const REQUIRED_REVIEWERS: &[(&str, &str)] = &[
    (
        crate::agent_roles::CANONICAL_AGENT_ROLES[0].name,
        crate::agent_roles::CANONICAL_AGENT_ROLES[0].manifest_path,
    ),
    (
        crate::agent_roles::CANONICAL_AGENT_ROLES[1].name,
        crate::agent_roles::CANONICAL_AGENT_ROLES[1].manifest_path,
    ),
    (
        crate::agent_roles::CANONICAL_AGENT_ROLES[5].name,
        crate::agent_roles::CANONICAL_AGENT_ROLES[5].manifest_path,
    ),
    (
        crate::agent_roles::CANONICAL_AGENT_ROLES[2].name,
        crate::agent_roles::CANONICAL_AGENT_ROLES[2].manifest_path,
    ),
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    #[test]
    fn registry_reviewers_are_exact_canonical_review_subset() {
        let got = super::REQUIRED_REVIEWERS
            .iter()
            .map(|(role, path)| {
                assert_eq!(
                    crate::agent_roles::by_name(role).unwrap().manifest_path,
                    *path
                );
                *role
            })
            .collect::<BTreeSet<_>>();
        let expected = [
            "claim-falsifier",
            "orchestration-recovery-reviewer",
            "security-reviewer",
            "product-journey-reviewer",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        assert_eq!(got, expected);
        assert!(!got.contains("repo-recon"));
        assert!(!got.contains("research-verifier"));
    }
}
