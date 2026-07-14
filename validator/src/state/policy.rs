use super::catalog::{DependencyActionSpec, DependencyStatus, HostGoalStatus, RuntimeField};
use super::limits::{valid_id, valid_relative_path, valid_text};
use super::product_state::{CeilingReduction, Scope};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn validate_structure(spec: &DependencyActionSpec) -> Vec<String> {
    let mut problems = Vec::new();
    require_id(
        &spec.expected_context_id,
        "expected-context-id",
        &mut problems,
    );
    require_id(
        &spec.expected_authority_catalog_id,
        "expected-authority-catalog-id",
        &mut problems,
    );
    validate_runtime_and_host(spec, &mut problems);
    duplicate_ids(
        spec.claims.iter().map(|item| item.claim_id.as_str()),
        "claim",
        &mut problems,
    );
    duplicate_ids(
        spec.inventory_policies
            .iter()
            .map(|item| item.code.as_str()),
        "inventory-policy",
        &mut problems,
    );
    duplicate_ids(
        spec.capability_requirements
            .iter()
            .map(|item| item.capability.as_str()),
        "capability-requirement",
        &mut problems,
    );
    duplicate_ids(
        spec.commands.iter().map(|item| item.command_id.as_str()),
        "command",
        &mut problems,
    );
    duplicate_ids(
        spec.actions.iter().map(|item| item.action_id.as_str()),
        "action",
        &mut problems,
    );
    duplicate_observations(spec, &mut problems);
    let runtime_fields = spec
        .runtime_requirements
        .iter()
        .map(|item| item.field.name())
        .collect::<Vec<_>>();
    duplicate_ids(runtime_fields, "runtime-requirement", &mut problems);

    let claims = claim_dimensions(spec, &mut problems);
    let mut repairs = BTreeMap::new();
    for fact in &spec.dependencies {
        require_id(&fact.dependency_id, "dependency", &mut problems);
        require_id(&fact.observation_id, "observation", &mut problems);
        validate_scope(&fact.scope, &mut problems);
        require_text(&fact.cause, "dependency-cause", &mut problems);
        require_impacts(
            &fact.ceiling_reductions,
            &claims,
            "dependency",
            &mut problems,
        );
        if fact.status == DependencyStatus::Satisfied && fact.repair.is_some() {
            problems.push("satisfied-dependency-has-repair".to_owned());
        }
        if fact.status != DependencyStatus::Satisfied && fact.repair.is_none() {
            problems.push("unsatisfied-dependency-missing-repair".to_owned());
        }
        if let Some(repair) = &fact.repair {
            super::policy_repair::register(repair, &mut repairs, &claims, &mut problems);
        }
    }
    for policy in &spec.inventory_policies {
        require_id(&policy.code, "inventory-code", &mut problems);
        require_id(&policy.scope_surface, "inventory-scope", &mut problems);
        require_impacts(
            &policy.ceiling_reductions,
            &claims,
            "inventory-policy",
            &mut problems,
        );
        super::policy_repair::register(&policy.repair, &mut repairs, &claims, &mut problems);
    }
    for requirement in &spec.capability_requirements {
        require_id(&requirement.capability, "capability", &mut problems);
        validate_scope(&requirement.scope, &mut problems);
        require_impacts(
            &requirement.ceiling_reductions,
            &claims,
            "capability",
            &mut problems,
        );
        super::policy_repair::register(&requirement.repair, &mut repairs, &claims, &mut problems);
    }
    for requirement in &spec.runtime_requirements {
        validate_scope(&requirement.scope, &mut problems);
        require_impacts(
            &requirement.ceiling_reductions,
            &claims,
            "runtime",
            &mut problems,
        );
        super::policy_repair::register(&requirement.repair, &mut repairs, &claims, &mut problems);
    }
    super::policy_action::validate(spec, &repairs, &mut problems);
    problems.sort();
    problems.dedup();
    problems
}

fn claim_dimensions<'a>(
    spec: &'a DependencyActionSpec,
    problems: &mut Vec<String>,
) -> BTreeMap<&'a str, BTreeSet<&'a str>> {
    let mut claims = BTreeMap::new();
    for claim in &spec.claims {
        require_id(&claim.claim_id, "claim", problems);
        if claim.maximum_dimensions.is_empty() {
            problems.push("empty-claim-dimensions".to_owned());
        }
        for dimension in &claim.maximum_dimensions {
            require_id(dimension, "claim-dimension", problems);
        }
        claims.entry(claim.claim_id.as_str()).or_insert_with(|| {
            claim
                .maximum_dimensions
                .iter()
                .map(String::as_str)
                .collect()
        });
    }
    claims
}

fn require_impacts(
    reductions: &[CeilingReduction],
    claims: &BTreeMap<&str, BTreeSet<&str>>,
    label: &str,
    problems: &mut Vec<String>,
) {
    if reductions.is_empty() {
        problems.push(format!("empty-actionable-claim-impact:{label}"));
    }
    for reduction in reductions {
        let Some(maximum) = claims.get(reduction.claim_id.as_str()) else {
            problems.push("reduction-unknown-claim".to_owned());
            continue;
        };
        if reduction.dimensions.is_empty() {
            problems.push("empty-ceiling-reduction".to_owned());
        }
        for dimension in &reduction.dimensions {
            if !maximum.contains(dimension.as_str()) {
                problems.push("reduction-unknown-dimension".to_owned());
            }
        }
    }
}

fn validate_runtime_and_host(spec: &DependencyActionSpec, problems: &mut Vec<String>) {
    for field in RuntimeField::ALL {
        if let Some(item) = spec.runtime_metadata.value(field)
            && (!valid_text(item.value(), false)
                || !valid_text(item.exposed_source(), false)
                || item.context_id() != spec.expected_context_id)
        {
            problems.push("invalid-runtime-provenance".to_owned());
        }
    }
    if spec.host_goal.status() != HostGoalStatus::Unavailable
        && (spec.host_goal.context_id() != Some(spec.expected_context_id.as_str())
            || spec
                .host_goal
                .exposed_source()
                .is_none_or(|source| !valid_text(source, false)))
    {
        problems.push("invalid-host-goal-provenance".to_owned());
    }
}

fn validate_scope(scope: &Scope, problems: &mut Vec<String>) {
    require_id(&scope.surface, "scope-surface", problems);
    if scope
        .relative_path
        .as_deref()
        .is_some_and(|path| !valid_relative_path(path))
    {
        problems.push("invalid-scope-relative-path".to_owned());
    }
}

fn require_id(value: &str, label: &str, problems: &mut Vec<String>) {
    if !valid_id(value) {
        problems.push(format!("invalid-{label}"));
    }
}

fn require_text(value: &str, label: &str, problems: &mut Vec<String>) {
    if !valid_text(value, false) {
        problems.push(format!("invalid-{label}"));
    }
}

fn duplicate_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    kind: &str,
    problems: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            problems.push(format!("duplicate-{kind}"));
        }
    }
}

fn duplicate_observations(spec: &DependencyActionSpec, problems: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for fact in &spec.dependencies {
        if !seen.insert((fact.dependency_id.as_str(), fact.observation_id.as_str())) {
            problems.push("duplicate-dependency-observation".to_owned());
        }
    }
}
