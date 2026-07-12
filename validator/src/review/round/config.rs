use crate::agent_roles::AgentRole;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) const MATERIALITY_FULL: &str = "FULL_SCOPE_MATERIAL_REVIEW_REQUIRED";
pub(crate) const MATERIALITY_DELTA: &str = "DELTA_REVIEW_ALLOWED";
pub(crate) const MATERIALITY_ADVISORY: &str = "ADVISORY_REVIEW_ALLOWED";
pub(crate) const MATERIALITY_BLOCKED: &str = "BLOCKED_BEFORE_REVIEW";

const PREFLIGHT_GATES: &[&str] = &[
    "anchor_existence_and_digest",
    "validator_receipt_status",
    "reviewer_registry_role_exposure",
    "stale_receipt_check",
    "package_private_artifact_hygiene",
    "readiness_validator",
    "claim_ceiling_check",
    "proof_surface_substitution_check",
];

pub(crate) struct ReviewRoleSpec {
    pub(crate) role_name: &'static str,
    pub(crate) agent_manifest_path: &'static str,
    pub(crate) focus_path: &'static str,
}

pub(crate) const REVIEW_ROLES: &[ReviewRoleSpec] = &[
    ReviewRoleSpec {
        role_name: crate::agent_roles::CANONICAL_AGENT_ROLES[0].name,
        agent_manifest_path: crate::agent_roles::CANONICAL_AGENT_ROLES[0].manifest_path,
        focus_path: "docs/hypercritical-review-law.md",
    },
    ReviewRoleSpec {
        role_name: crate::agent_roles::CANONICAL_AGENT_ROLES[1].name,
        agent_manifest_path: crate::agent_roles::CANONICAL_AGENT_ROLES[1].manifest_path,
        focus_path: "validator/src/review/round/registry.rs",
    },
    ReviewRoleSpec {
        role_name: crate::agent_roles::CANONICAL_AGENT_ROLES[5].name,
        agent_manifest_path: crate::agent_roles::CANONICAL_AGENT_ROLES[5].manifest_path,
        focus_path: "validator/src/schema_catalog/mod.rs",
    },
    ReviewRoleSpec {
        role_name: crate::agent_roles::CANONICAL_AGENT_ROLES[2].name,
        agent_manifest_path: crate::agent_roles::CANONICAL_AGENT_ROLES[2].manifest_path,
        focus_path: "docs/plugin-resource-map.md",
    },
];

pub(crate) fn review_role_spec(role: &str) -> Option<&'static ReviewRoleSpec> {
    REVIEW_ROLES.iter().find(|spec| spec.role_name == role)
}

pub(crate) fn agent_role(spec: &ReviewRoleSpec) -> AgentRole {
    crate::agent_roles::by_name(spec.role_name)
        .expect("review roles must project from the adopted canonical role table")
}

pub(crate) fn expected_result(review_stage: &str) -> &'static str {
    let _ = review_stage;
    "no_material_finding"
}

pub(crate) fn full_anchor_required(review_stage: &str, anchor_policy: &str) -> bool {
    review_stage == "falsification" && anchor_policy == "validator_review_target_archive"
}

pub(crate) struct MaterialityDerivation {
    pub(crate) decision: &'static str,
    pub(crate) errors: Vec<String>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum MaterialityAuthority {
    StaticFixture,
    LiveObservationUnavailable,
    CanonicalSourceUnavailable,
}

#[derive(Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Trigger {
    MaterialClaimBoundary,
    PhaseAdvancement,
    ProofAnchorChange,
    ReviewerRegistryChange,
    PersonaContractChange,
    MaterialCodeRuntimeChange,
    PriorReviseOrBlockedRepair,
    RegeneratedAnchorRepair,
    SecurityPrivacyTrustBoundaryChange,
    PackageRuntimeVisibilitySurfaceChange,
    StaleAnchor,
    NarrowValidatorOnlyChange,
    AdvisoryOnlyNoCandidateChange,
}

pub(crate) fn derive_materiality(
    value: &Value,
    identities_ok: bool,
    authority: MaterialityAuthority,
) -> MaterialityDerivation {
    if authority != MaterialityAuthority::StaticFixture {
        let mut errors = vec![
            "materiality_candidate_change_observation_unavailable".into(),
            "materiality_executed_gate_observation_unavailable".into(),
        ];
        if authority == MaterialityAuthority::CanonicalSourceUnavailable {
            errors.push("materiality_canonical_anchor_source_unavailable".into());
        }
        return MaterialityDerivation {
            decision: MATERIALITY_BLOCKED,
            errors,
        };
    }
    let mut errors = Vec::new();
    let gates_ok = gates_valid(value, &mut errors);
    let triggers = triggers(value, &mut errors);
    let repairs_ok = bounded_strings(value, "validator_repairs_recommended");
    if !repairs_ok {
        errors.push("materiality_repairs_invalid".into());
    }
    let decision = if !identities_ok || !gates_ok || !repairs_ok || triggers.is_empty() {
        MATERIALITY_BLOCKED
    } else if !strings(value, "validator_repairs_recommended").is_empty()
        || triggers.contains(&Trigger::StaleAnchor)
    {
        MATERIALITY_BLOCKED
    } else if triggers.iter().any(|trigger| {
        !matches!(
            trigger,
            Trigger::NarrowValidatorOnlyChange | Trigger::AdvisoryOnlyNoCandidateChange
        )
    }) {
        MATERIALITY_FULL
    } else if triggers == [Trigger::NarrowValidatorOnlyChange] {
        MATERIALITY_DELTA
    } else if triggers == [Trigger::AdvisoryOnlyNoCandidateChange] {
        MATERIALITY_ADVISORY
    } else {
        MATERIALITY_BLOCKED
    };
    MaterialityDerivation { decision, errors }
}

fn triggers(value: &Value, errors: &mut Vec<String>) -> Vec<Trigger> {
    let Some(raw) = value.get("material_triggers").and_then(Value::as_array) else {
        errors.push("materiality_trigger_missing".into());
        return Vec::new();
    };
    if raw.is_empty() || raw.len() > 16 {
        errors.push("materiality_trigger_missing".into());
        return Vec::new();
    }
    serde_json::from_value(Value::Array(raw.clone())).unwrap_or_else(|_| {
        errors.push("materiality_trigger_invalid".into());
        Vec::new()
    })
}

fn gates_valid(value: &Value, errors: &mut Vec<String>) -> bool {
    let required = strings(value, "deterministic_gates_required");
    let run = strings(value, "deterministic_gates_run");
    let required_len = value["deterministic_gates_required"]
        .as_array()
        .map(Vec::len);
    let run_len = value["deterministic_gates_run"].as_array().map(Vec::len);
    let expected = PREFLIGHT_GATES.iter().copied().collect::<BTreeSet<_>>();
    let required_ok = required_len == Some(PREFLIGHT_GATES.len())
        && required.iter().map(String::as_str).collect::<BTreeSet<_>>() == expected;
    if !required_ok {
        errors.push("materiality_required_gate_missing".into());
    }
    if run != required || run_len != required_len {
        errors.push("materiality_required_gate_not_run".into());
    }
    required_ok && run == required && run_len == required_len
}

fn bounded_strings(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|rows| {
            rows.len() <= 16
                && rows.iter().all(|row| {
                    row.as_str().is_some_and(|text| {
                        !text.trim().is_empty()
                            && text.len() <= 256
                            && !text.chars().any(char::is_control)
                    })
                })
        })
}

fn strings(value: &Value, key: &str) -> BTreeSet<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}
