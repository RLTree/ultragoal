use super::fixture::*;
use crate::context::EffectClass;
use crate::inventory::FindingSeverity as InventorySeverity;
use crate::state::catalog::{
    CommandBinding, DependencyActionCatalog, DependencyFact, DependencyStatus, FactAuthority,
};
use crate::state::engine::derive_bound;
use crate::state::snapshot::InventoryObservation;
use crate::state::types::{AuthorityRequirement, StateError};

fn spec_with_missing() -> crate::state::catalog::DependencyActionSpec {
    let mut spec = spec();
    spec.dependencies.push(DependencyFact {
        dependency_id: "dep-a".to_owned(),
        observation_id: "probe-a".to_owned(),
        status: DependencyStatus::Missing,
        authority: FactAuthority::DirectProbe,
        scope: scope("runtime"),
        cause: "dependency is missing".to_owned(),
        repair: Some(repair(
            "repair-dep-a",
            AuthorityRequirement::None,
            EffectClass::Read,
        )),
        ceiling_reductions: reduction(&["runtime"]),
    });
    spec
}

fn invalid(spec: crate::state::catalog::DependencyActionSpec) -> String {
    DependencyActionCatalog::from_untrusted_spec(spec)
        .unwrap_err()
        .to_string()
}

#[test]
fn proof_shaped_and_path_like_repair_targets_are_rejected() {
    for target in [
        "receipt-authority",
        "telemetry-authority",
        "generated-view",
        "current-state-view",
        "status-label",
        "status",
        "view",
        "proof",
        "claim-witness",
        "evidence-artifact",
        "/absolute/path",
        "../traversal",
    ] {
        let mut spec = spec_with_missing();
        spec.dependencies[0].repair.as_mut().unwrap().target.id = target.to_owned();
        let error = invalid(spec);
        assert!(
            error.contains("repair-target") || error.contains("repair-shape"),
            "{target}: {error}"
        );
    }

    for path in [
        "/absolute/path",
        "../traversal",
        "C:\\absolute\\path",
        "\\rooted-on-current-drive",
        "\\\\server\\share",
    ] {
        let mut spec = spec_with_missing();
        spec.dependencies[0].scope.relative_path = Some(path.to_owned());
        assert!(invalid(spec).contains("invalid-scope-relative-path"));
    }
}

#[test]
fn control_bidi_osc_nul_and_oversized_strings_are_rejected() {
    for poisoned in [
        "line\nbreak".to_owned(),
        "bidi\u{202e}override".to_owned(),
        "osc\u{1b}]8;;file:///tmp\u{7}".to_owned(),
        "nul\0byte".to_owned(),
        "x".repeat(super::super::limits::MAX_TEXT_BYTES + 1),
    ] {
        let mut spec = spec_with_missing();
        spec.dependencies[0].repair.as_mut().unwrap().summary = poisoned;
        assert!(invalid(spec).contains("invalid-repair-shape"));
    }
}

#[test]
fn invalid_catalog_errors_do_not_echo_untrusted_control_text() {
    let canary = "CATALOG-CANARY\u{1b}]8;;file:///tmp/pwn\u{7}\u{202e}\n";
    let mut spec = spec_with_missing();
    spec.dependencies[0].repair.as_mut().unwrap().repair_id = canary.to_owned();
    spec.dependencies[0].repair.as_mut().unwrap().summary = canary.to_owned();
    spec.actions.push(command(canary, canary, 1));

    let error = invalid(spec);
    assert!(!error.contains("CATALOG-CANARY"), "{error:?}");
    assert!(!error.contains('\u{1b}'), "{error:?}");
    assert!(!error.contains('\u{202e}'), "{error:?}");
    assert!(!error.contains('\n'), "{error:?}");
}

#[test]
fn commands_are_canonical_bindings_and_effect_mismatch_is_rejected() {
    for argv in [
        vec![
            "ultragoal".to_owned(),
            "inspect".to_owned(),
            "--format=json".to_owned(),
        ],
        vec![
            "ultragoal".to_owned(),
            "inspect".to_owned(),
            "--json".to_owned(),
        ],
    ] {
        let mut noncanonical_json = spec();
        noncanonical_json
            .commands
            .iter_mut()
            .find(|command| command.command_id == "inspect-json")
            .unwrap()
            .argv = argv;
        assert!(invalid(noncanonical_json).contains("missing-canonical-inspect-json-command"));
    }

    let mut mismatch = spec_with_missing();
    mismatch.commands.push(CommandBinding {
        command_id: "workspace-write".to_owned(),
        argv: vec!["ultragoal".to_owned(), "fit".to_owned(), "apply".to_owned()],
        effect: EffectClass::WorkspaceWrite,
    });
    let mut action = command("mismatched-action", "repair-dep-a", 1);
    action.command_id = Some("workspace-write".to_owned());
    mismatch.actions.push(action);
    assert!(invalid(mismatch).contains("command-effect-mismatch"));

    for argv in [
        vec!["/absolute/executable".to_owned()],
        vec!["ultragoal".to_owned(), "../escape".to_owned()],
        vec![
            "ultragoal".to_owned(),
            "check".to_owned(),
            "--target=/absolute".to_owned(),
        ],
        vec![
            "ultragoal".to_owned(),
            "check".to_owned(),
            "\\\\server\\share".to_owned(),
        ],
    ] {
        let mut spec = spec();
        spec.commands.push(CommandBinding {
            command_id: "unsafe-command".to_owned(),
            argv,
            effect: EffectClass::Read,
        });
        assert!(invalid(spec).contains("invalid-command-binding"));
    }
}

#[test]
fn actionable_inputs_without_claim_impacts_are_rejected() {
    let mut spec = spec_with_missing();
    spec.dependencies[0].ceiling_reductions.clear();
    assert!(invalid(spec).contains("empty-actionable-claim-impact"));
}

#[test]
fn catalog_input_and_projection_resource_limits_fail_closed() {
    let mut too_many = spec();
    too_many.claims = (0..=super::super::limits::MAX_CLAIMS)
        .map(|index| crate::state::catalog::ClaimSpec {
            claim_id: format!("CL-{index}"),
            maximum_dimensions: vec!["source".to_owned()],
        })
        .collect();
    assert!(matches!(
        DependencyActionCatalog::from_untrusted_spec(too_many),
        Err(StateError::ResourceLimit(_))
    ));

    let projection = vec![b'x'; super::super::limits::MAX_PROJECTION_BYTES + 1];
    assert!(matches!(
        super::super::limits::bounded_projection(projection),
        Err(StateError::ResourceLimit(_))
    ));

    let mut oversized_bytes = spec();
    let large_argument = "a".repeat(super::super::limits::MAX_TEXT_BYTES);
    for index in 0..(super::super::limits::MAX_COMMANDS - oversized_bytes.commands.len()) {
        oversized_bytes.commands.push(CommandBinding {
            command_id: format!("bulk-command-{index}"),
            argv: vec![
                "ultragoal".to_owned(),
                "check".to_owned(),
                large_argument.clone(),
            ],
            effect: EffectClass::Read,
        });
    }
    assert!(matches!(
        DependencyActionCatalog::from_untrusted_spec(oversized_bytes),
        Err(StateError::ResourceLimit(_))
    ));
}

#[test]
fn oversized_authority_finding_sets_and_control_canaries_fail_before_projection() {
    let mut too_many = inputs();
    let row = InventoryObservation {
        code: "bounded-finding".to_owned(),
        severity: InventorySeverity::Error,
        entry_id: None,
        relative_path: None,
        cause: "bounded".to_owned(),
    };
    too_many.inventory_findings = vec![row; super::super::limits::MAX_INVENTORY_FINDINGS + 1];
    assert!(matches!(
        derive_bound(too_many, &catalog(spec())),
        Err(StateError::ResourceLimit(_))
    ));

    let mut too_many_capabilities = inputs();
    too_many_capabilities.capabilities = (0..=super::super::limits::MAX_CAPABILITIES)
        .map(|index| (format!("capability-{index}"), true))
        .collect();
    assert!(matches!(
        derive_bound(too_many_capabilities, &catalog(spec())),
        Err(StateError::ResourceLimit(_))
    ));

    let mut poisoned = inputs();
    poisoned.inventory_findings.push(InventoryObservation {
        code: "poisoned-finding".to_owned(),
        severity: InventorySeverity::Error,
        entry_id: None,
        relative_path: None,
        cause: "osc\u{1b}]8;;file:///tmp\u{7}".to_owned(),
    });
    assert!(matches!(
        derive_bound(poisoned, &catalog(spec())),
        Err(StateError::InvalidCatalog(_))
    ));
}
