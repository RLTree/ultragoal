#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CommandObservabilitySpec {
    pub(crate) id: &'static str,
    pub(crate) family: &'static str,
    pub(crate) command_args: &'static [&'static str],
    pub(crate) operation: &'static str,
    pub(crate) receipt_rel: &'static str,
    pub(crate) validator_check_id: &'static str,
    pub(crate) claim_impact: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SurfaceObservabilitySpec {
    pub(crate) family: &'static str,
    pub(crate) command_ids: &'static [&'static str],
    pub(crate) owner_surface: &'static str,
    pub(crate) claim_impact: &'static str,
}

const PACKAGE_COMMANDS: &[&str] = &["package digest"];
const INSTALL_CACHE_COMMANDS: &[&str] = &["install audit"];

const COMMAND_SPECS: &[CommandObservabilitySpec] = &[
    CommandObservabilitySpec {
        id: "package digest",
        family: "package",
        command_args: &["package", "digest"],
        operation: "package.digest",
        receipt_rel: "validation_artifacts/observability/package-digest.json",
        validator_check_id: "package-digest-observability-binding",
        claim_impact: "supports_source_package_digest_only_no_readiness_release_completion_update_goal",
    },
    CommandObservabilitySpec {
        id: "install audit",
        family: "install-cache",
        command_args: &[
            "install",
            "audit",
            "--receipt",
            "validation_artifacts/cli/install-audit-receipt.json",
        ],
        operation: "install_audit",
        receipt_rel: "validation_artifacts/cli/install-audit-receipt.json",
        validator_check_id: "install-audit-observability-binding",
        claim_impact: "supports_install_audit_observability_roundtrip_only_no_install_refresh_no_readiness_release_completion_update_goal",
    },
];

const SURFACE_SPECS: &[SurfaceObservabilitySpec] = &[
    SurfaceObservabilitySpec {
        family: "package",
        command_ids: PACKAGE_COMMANDS,
        owner_surface: "command-family:package",
        claim_impact: "source_package_observability_command_roundtrip_only_not_observability_product_closure",
    },
    SurfaceObservabilitySpec {
        family: "install-cache",
        command_ids: INSTALL_CACHE_COMMANDS,
        owner_surface: "command-family:install-cache",
        claim_impact: "install_cache_observability_command_roundtrip_only_not_install_cache_refresh_or_claim_closure",
    },
];

pub(crate) fn command(id: &str) -> Option<CommandObservabilitySpec> {
    let normalized = normalize_id(id);
    COMMAND_SPECS
        .iter()
        .copied()
        .find(|spec| normalize_id(spec.id) == normalized)
}

pub(crate) fn family(id: &str) -> Option<SurfaceObservabilitySpec> {
    let normalized = normalize_id(id);
    SURFACE_SPECS
        .iter()
        .copied()
        .find(|spec| normalize_id(spec.family) == normalized)
}

pub(crate) fn family_commands(family: SurfaceObservabilitySpec) -> Vec<CommandObservabilitySpec> {
    family
        .command_ids
        .iter()
        .filter_map(|id| command(id))
        .collect()
}

pub(crate) fn command_ids() -> Vec<&'static str> {
    COMMAND_SPECS.iter().map(|spec| spec.id).collect()
}

fn normalize_id(id: &str) -> String {
    id.trim().replace('-', " ").replace('_', " ")
}

#[cfg(test)]
mod tests {
    use super::{command, family, family_commands};

    #[test]
    fn command_specs_accept_operator_spellings() {
        let spec = command("package-digest").expect("package spec");
        assert_eq!(spec.id, "package digest");
        assert_eq!(spec.command_args, &["package", "digest"]);
        assert_eq!(spec.operation, "package.digest");
        let install = command("install-audit").expect("install spec");
        assert_eq!(install.id, "install audit");
        assert_eq!(install.family, "install-cache");
        assert_eq!(
            install.command_args,
            &[
                "install",
                "audit",
                "--receipt",
                "validation_artifacts/cli/install-audit-receipt.json"
            ]
        );
        assert_eq!(install.operation, "install_audit");
    }

    #[test]
    fn family_specs_derive_commands_without_generated_row_edits() {
        let package_family = family("package").expect("package family");
        let commands = family_commands(package_family);
        assert_eq!(package_family.owner_surface, "command-family:package");
        assert_eq!(
            commands.iter().map(|spec| spec.id).collect::<Vec<_>>(),
            vec!["package digest"]
        );
        let install_cache = family("install-cache").expect("install cache family");
        let commands = family_commands(install_cache);
        assert_eq!(install_cache.owner_surface, "command-family:install-cache");
        assert_eq!(
            commands.iter().map(|spec| spec.id).collect::<Vec<_>>(),
            vec!["install audit"]
        );
    }

    #[test]
    fn unknown_specs_do_not_become_observable() {
        assert!(command("final-packet prove").is_none());
        assert!(family("unknown").is_none());
    }
}
