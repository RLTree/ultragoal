use super::*;

/// Current product routes intentionally carry no adopted-handoff
/// configuration. Retained inventory/package routes use the explicitly named
/// compatibility contexts below.
pub(crate) fn current_read_context(root: &Path) -> Result<LiveContext, ()> {
    LiveContext::build(BuildRequest::new(root).with_effect(EffectClass::Read)).map_err(|_| ())
}

pub(crate) fn current_capabilities_context(root: &Path) -> Result<LiveContext, ()> {
    LiveContext::build(
        BuildRequest::new(root)
            .with_effect(EffectClass::Read)
            .probe_tool("codex"),
    )
    .map_err(|_| ())
}

pub(crate) fn current_workspace_context(root: &Path) -> Result<LiveContext, ()> {
    LiveContext::build(BuildRequest::new(root).with_root_workspace_grant(root)).map_err(|_| ())
}

pub(crate) fn compatibility_read_context(root: &Path) -> Result<LiveContext, ()> {
    LiveContext::build(
        BuildRequest::new(root)
            .with_effect(EffectClass::Read)
            .bind_non_secret_configuration(
                ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
                ADOPTED_HANDOFF_MANIFEST_SHA256,
            ),
    )
    .map_err(|_| ())
}

pub(crate) fn compatibility_workspace_context(root: &Path) -> Result<LiveContext, ()> {
    LiveContext::build(
        BuildRequest::new(root)
            .with_root_workspace_grant(root)
            .bind_non_secret_configuration(
                ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
                ADOPTED_HANDOFF_MANIFEST_SHA256,
            ),
    )
    .map_err(|_| ())
}

#[cfg(test)]
pub(crate) fn read_context(root: &Path) -> Result<LiveContext, ()> {
    compatibility_read_context(root)
}

#[cfg(test)]
mod tests {
    use super::super::repository_fixture::Repository;
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn current_fit_contexts_ignore_removed_legacy_authority_inputs() {
        let root = minimal_repository("current-fit-context");

        let read = current_read_context(&root).unwrap();
        let workspace = current_workspace_context(&root).unwrap();

        assert!(read.configuration().public_values.is_empty());
        assert!(workspace.configuration().public_values.is_empty());
        assert_eq!(read.effect().selected, EffectClass::Read);
        assert_eq!(workspace.effect().selected, EffectClass::WorkspaceWrite);
        assert!(
            workspace
                .effect()
                .write_scopes
                .iter()
                .any(|scope| Path::new(scope) == root)
        );
        assert!(!root.join("docs").exists());
        assert!(!root.join("migration").exists());
        assert!(!root.join("validation_artifacts").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compatibility_contexts_retain_the_adopted_handoff_binding() {
        let repository = Repository::new("compatibility-context");

        for context in [
            compatibility_read_context(&repository.root).unwrap(),
            compatibility_workspace_context(&repository.root).unwrap(),
        ] {
            assert_eq!(
                context
                    .configuration()
                    .public_values
                    .get(ADOPTED_HANDOFF_DIGEST_CONFIG_KEY),
                Some(&ADOPTED_HANDOFF_MANIFEST_SHA256.to_owned())
            );
        }
    }

    fn minimal_repository(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-current-context-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "current@example.invalid"]);
        git(&root, &["config", "user.name", "Current Context"]);
        fs::write(root.join("README.md"), b"current product fixture\n").unwrap();
        git(&root, &["add", "README.md"]);
        git(&root, &["commit", "-qm", "minimal current fixture"]);
        root
    }

    fn git(root: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .args(arguments)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .current_dir(root)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {arguments:?}: {output:?}");
    }
}

pub(crate) fn inventory_outcome(inventory: &AuthorityCatalog) -> RuntimeOutcome {
    let exit = if inventory.has_error_findings() {
        ExitClass::ActionableFinding
    } else {
        ExitClass::Success
    };
    match inventory.to_canonical_json() {
        Ok(machine) if public_output_allowed(machine.len()) => RuntimeOutcome::payload(
            exit,
            machine,
            format!(
                "inventory {} entries={} findings={}",
                inventory.catalog_id(),
                inventory.entries().len(),
                inventory.findings().len()
            ),
        ),
        Ok(_) => failure(
            DiagnosticId::InventoryUnavailable,
            "the canonical authority inventory exceeds the bounded public output",
            "authority inventory output",
            "narrow the repository surface or repair unbounded authority discovery before retrying",
            "inventory and dependent claims remain unavailable",
        ),
        Err(_) => inventory_unavailable(),
    }
}

pub(crate) fn context_unavailable() -> RuntimeOutcome {
    failure(
        DiagnosticId::ContextUnavailable,
        "a current candidate-bound LiveContext could not be constructed",
        "live repository context",
        "verify the root and retry against one stable Git candidate",
        "context and dependent claims remain unavailable",
    )
}

pub(crate) fn inventory_unavailable() -> RuntimeOutcome {
    failure(
        DiagnosticId::InventoryUnavailable,
        "the canonical authority inventory could not be derived from the current context",
        "authority inventory",
        "repair the adopted registry or concurrent candidate mutation and rerun inspection",
        "inventory and dependent claims remain unavailable",
    )
}

pub(crate) fn inventory_failure(error: &crate::inventory::InventoryError) -> RuntimeOutcome {
    let (cause, repair) = match error {
        crate::inventory::InventoryError::Activation(_) => (
            "the adopted activation registry conflicts with the compiled command authority",
            "reconcile the generated activation authority before retrying package work",
        ),
        crate::inventory::InventoryError::Context(_) => (
            "the candidate changed while the authority inventory was being read",
            "retry against one stable candidate",
        ),
        _ => (
            "the canonical authority inventory could not be derived from the current context",
            "repair the adopted registry or concurrent candidate mutation and rerun inspection",
        ),
    };
    failure(
        DiagnosticId::InventoryUnavailable,
        cause,
        "authority inventory",
        repair,
        "inventory and dependent claims remain unavailable",
    )
}

pub(crate) fn state_unavailable() -> RuntimeOutcome {
    failure(
        DiagnosticId::StateUnavailable,
        "the canonical typed state graph could not be derived from current authority inputs",
        "typed product state",
        "repair the current inventory or adopted state policy and rerun inspection",
        "state and completion claims remain unavailable",
    )
}

pub(crate) fn failure(
    id: DiagnosticId,
    cause: &'static str,
    surface: &'static str,
    repair: &'static str,
    ceiling: &'static str,
) -> RuntimeOutcome {
    RuntimeOutcome::failure(
        ExitClass::UnsupportedCapability,
        Diagnostic::new(
            id,
            ExitClass::UnsupportedCapability,
            DiagnosticDetails {
                cause,
                affected_surface: surface,
                repair,
                effect: "read",
                rerun: "ultragoal --json inspect context",
                ceiling,
            },
        ),
    )
}
