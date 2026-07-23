use super::*;

pub(crate) fn read_context(root: &Path) -> Result<LiveContext, ()> {
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

pub(crate) fn workspace_context(root: &Path) -> Result<LiveContext, ()> {
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
