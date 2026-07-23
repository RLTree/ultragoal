use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BehavioralRole {
    CliIngressAdapter,
    ClaimGuard,
    TestContext,
    PackageInputDescriptor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CanonicalTarget {
    StableId(&'static str),
    RelativePath(&'static str),
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BehavioralRoleBinding {
    pub role: BehavioralRole,
    pub relative_path: &'static str,
    pub canonical_target: CanonicalTarget,
}

const BINDINGS: [BehavioralRoleBinding; 10] = [
    binding(
        BehavioralRole::CliIngressAdapter,
        "validator/src/argument_parser/public_arguments.rs",
        CanonicalTarget::StableId("PS-CLI"),
    ),
    binding(
        BehavioralRole::CliIngressAdapter,
        "validator/src/argument_parser/mod.rs",
        CanonicalTarget::StableId("PS-CLI"),
    ),
    binding(
        BehavioralRole::ClaimGuard,
        "validator/src/audit/final_packet/mod.rs",
        CanonicalTarget::StableId("HCT-CLAIMS"),
    ),
    binding(
        BehavioralRole::ClaimGuard,
        "validator/src/audit/final_packet/observability/mod.rs",
        CanonicalTarget::StableId("HCT-CLAIMS"),
    ),
    binding(
        BehavioralRole::TestContext,
        "validator/src/audit/final_packet/observability/tests.rs",
        CanonicalTarget::None,
    ),
    binding(
        BehavioralRole::ClaimGuard,
        "validator/src/audit/final_packet/references/coverage.rs",
        CanonicalTarget::StableId("HCT-CLAIMS"),
    ),
    binding(
        BehavioralRole::ClaimGuard,
        "validator/src/audit/final_packet/references/mod.rs",
        CanonicalTarget::StableId("HCT-CLAIMS"),
    ),
    binding(
        BehavioralRole::ClaimGuard,
        "validator/src/audit/final_packet/references/package.rs",
        CanonicalTarget::StableId("HCT-CLAIMS"),
    ),
    binding(
        BehavioralRole::ClaimGuard,
        "validator/src/audit/final_packet/references/source_audit.rs",
        CanonicalTarget::StableId("HCT-CLAIMS"),
    ),
    binding(
        BehavioralRole::PackageInputDescriptor,
        "plugin-manifest-draft.json",
        CanonicalTarget::RelativePath(".codex-plugin/plugin.json"),
    ),
];

const fn binding(
    role: BehavioralRole,
    relative_path: &'static str,
    canonical_target: CanonicalTarget,
) -> BehavioralRoleBinding {
    BehavioralRoleBinding {
        role,
        relative_path,
        canonical_target,
    }
}

pub(crate) fn for_path(path: &Path) -> Option<&'static BehavioralRoleBinding> {
    BINDINGS
        .iter()
        .find(|binding| path == Path::new(binding.relative_path))
}

pub(crate) fn legacy_route_targets_active_role(
    stable_id: Option<&str>,
    relative_path: Option<&str>,
) -> bool {
    BINDINGS.iter().any(|binding| {
        relative_path == Some(binding.relative_path)
            || stable_id.is_some_and(|stable_id| {
                stable_id
                    .strip_prefix("LEGACY-COMMAND:")
                    .or_else(|| stable_id.strip_prefix("LEGACY-FINALIZER:"))
                    .or_else(|| stable_id.strip_prefix("LEGACY-MANIFEST-PROJECTION:"))
                    == Some(binding.relative_path)
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_roles_and_targets_are_closed_and_nonoverlapping() {
        assert_eq!(BINDINGS.len(), 10);
        let mut paths = std::collections::BTreeSet::new();
        for binding in BINDINGS {
            assert!(paths.insert(binding.relative_path));
            assert_eq!(for_path(Path::new(binding.relative_path)), Some(&binding));
        }
        assert_eq!(
            for_path(Path::new("validator/src/argument_parser/authority.rs")),
            None
        );
        assert_eq!(
            for_path(Path::new(
                "validator/src/audit/final_packet/observability/tests.rs"
            ))
            .map(|binding| binding.canonical_target),
            Some(CanonicalTarget::None)
        );
        assert_eq!(
            for_path(Path::new("plugin-manifest-draft.json"))
                .map(|binding| binding.canonical_target),
            Some(CanonicalTarget::RelativePath(".codex-plugin/plugin.json"))
        );
    }

    #[test]
    fn stale_legacy_route_spellings_cannot_demote_active_roles() {
        for binding in BINDINGS {
            assert!(legacy_route_targets_active_role(
                None,
                Some(binding.relative_path)
            ));
        }
        assert!(legacy_route_targets_active_role(
            Some("LEGACY-COMMAND:validator/src/argument_parser/mod.rs"),
            None
        ));
        assert!(!legacy_route_targets_active_role(
            Some("LEGACY-COMMAND:validator/src/argument_parser/authority.rs"),
            None
        ));
    }
}
