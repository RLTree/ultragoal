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

const COMMAND_SPECS: &[CommandObservabilitySpec] = &[CommandObservabilitySpec {
    id: "package digest",
    family: "package",
    command_args: &["package", "digest"],
    operation: "package.digest",
    receipt_rel: "validation_artifacts/observability/package-digest.json",
    validator_check_id: "package-digest-observability-binding",
    claim_impact: "supports_source_package_digest_only_no_readiness_release_completion_update_goal",
}];

const SURFACE_SPECS: &[SurfaceObservabilitySpec] = &[SurfaceObservabilitySpec {
    family: "package",
    command_ids: PACKAGE_COMMANDS,
    owner_surface: "command-family:package",
    claim_impact: "source_package_observability_command_roundtrip_only_not_gate92_closure",
}];

pub(crate) fn command(id: &str) -> Option<CommandObservabilitySpec> {
    let normalized = normalize_id(id);
    COMMAND_SPECS
        .iter()
        .copied()
        .find(|spec| spec.id == normalized)
}

pub(crate) fn family(id: &str) -> Option<SurfaceObservabilitySpec> {
    let normalized = normalize_id(id);
    SURFACE_SPECS
        .iter()
        .copied()
        .find(|spec| spec.family == normalized)
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
    use super::{command, command_ids, family, family_commands};

    #[test]
    fn command_specs_accept_operator_spellings() {
        let spec = command("package-digest").expect("package spec");
        assert_eq!(spec.id, "package digest");
        assert_eq!(spec.command_args, &["package", "digest"]);
        assert_eq!(spec.operation, "package.digest");
    }

    #[test]
    fn family_specs_derive_commands_without_generated_row_edits() {
        let family = family("package").expect("package family");
        let commands = family_commands(family);
        assert_eq!(family.owner_surface, "command-family:package");
        assert_eq!(
            commands.iter().map(|spec| spec.id).collect::<Vec<_>>(),
            command_ids()
        );
    }

    #[test]
    fn unknown_specs_are_not_silently_fitted() {
        assert!(command("final-packet prove").is_none());
        assert!(family("unknown").is_none());
    }
}
