use super::catalog::DependencyActionSpec;
use super::types::{AuthorityRequest, Repair, Scope, StateError};
use std::path::{Component, Path};

pub(crate) const MAX_CATALOG_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAX_PROJECTION_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const MAX_TEXT_BYTES: usize = 4096;
pub(crate) const MAX_IDENTIFIER_BYTES: usize = 160;
pub(crate) const MAX_CLAIMS: usize = 256;
pub(crate) const MAX_DEPENDENCIES: usize = 4096;
pub(crate) const MAX_POLICIES: usize = 4096;
pub(crate) const MAX_ACTIONS: usize = 4096;
pub(crate) const MAX_COMMANDS: usize = 1024;
pub(crate) const MAX_INVENTORY_FINDINGS: usize = 4096;
pub(crate) const MAX_CAPABILITIES: usize = 1024;
pub(crate) const MAX_ARGV: usize = 64;

pub(crate) fn preflight_catalog(spec: &DependencyActionSpec) -> Result<(), StateError> {
    for (label, actual, maximum) in [
        ("claims", spec.claims.len(), MAX_CLAIMS),
        ("dependencies", spec.dependencies.len(), MAX_DEPENDENCIES),
        (
            "inventory policies",
            spec.inventory_policies.len(),
            MAX_POLICIES,
        ),
        (
            "capability requirements",
            spec.capability_requirements.len(),
            MAX_CLAIMS,
        ),
        ("runtime requirements", spec.runtime_requirements.len(), 5),
        ("commands", spec.commands.len(), MAX_COMMANDS),
        ("actions", spec.actions.len(), MAX_ACTIONS),
    ] {
        if actual > maximum {
            return Err(StateError::ResourceLimit(format!(
                "{label} count {actual} exceeds {maximum}"
            )));
        }
    }
    if estimated_catalog_bytes(spec) > MAX_CATALOG_BYTES {
        return Err(StateError::ResourceLimit(
            "dependency/action catalog estimated bytes".to_owned(),
        ));
    }
    Ok(())
}

fn estimated_catalog_bytes(spec: &DependencyActionSpec) -> usize {
    let mut total = 1024usize;
    add(&mut total, &spec.expected_context_id);
    add(&mut total, &spec.expected_authority_catalog_id);
    for claim in &spec.claims {
        add(&mut total, &claim.claim_id);
        add_many(&mut total, &claim.maximum_dimensions);
    }
    for fact in &spec.dependencies {
        add(&mut total, &fact.dependency_id);
        add(&mut total, &fact.observation_id);
        add_scope(&mut total, &fact.scope);
        add(&mut total, &fact.cause);
        if let Some(repair) = &fact.repair {
            add_repair(&mut total, repair);
        }
        add_reductions(&mut total, &fact.ceiling_reductions);
    }
    for policy in &spec.inventory_policies {
        add(&mut total, &policy.code);
        add(&mut total, &policy.scope_surface);
        add_repair(&mut total, &policy.repair);
        add_reductions(&mut total, &policy.ceiling_reductions);
    }
    for requirement in &spec.capability_requirements {
        add(&mut total, &requirement.capability);
        add_scope(&mut total, &requirement.scope);
        add_repair(&mut total, &requirement.repair);
        add_reductions(&mut total, &requirement.ceiling_reductions);
    }
    for requirement in &spec.runtime_requirements {
        add_scope(&mut total, &requirement.scope);
        add_repair(&mut total, &requirement.repair);
        add_reductions(&mut total, &requirement.ceiling_reductions);
    }
    for field in super::catalog::RuntimeField::ALL {
        if let Some(value) = spec.runtime_metadata.value(field) {
            add(&mut total, value.value());
            add(&mut total, value.exposed_source());
            add(&mut total, value.context_id());
        }
    }
    for command in &spec.commands {
        add(&mut total, &command.command_id);
        add_many(&mut total, &command.argv);
    }
    for action in &spec.actions {
        add(&mut total, &action.action_id);
        add(&mut total, &action.repair_id);
        add_many(&mut total, &action.requires_dependencies);
        add_many(&mut total, &action.required_capabilities);
        if let Some(command_id) = &action.command_id {
            add(&mut total, command_id);
        }
        if let Some(request) = &action.authority_request {
            add_request(&mut total, request);
        }
    }
    if let Some(source) = spec.host_goal.exposed_source() {
        add(&mut total, source);
    }
    if let Some(context_id) = spec.host_goal.context_id() {
        add(&mut total, context_id);
    }
    total
}

fn add_repair(total: &mut usize, repair: &Repair) {
    add(total, &repair.repair_id);
    add(total, &repair.target.id);
    add(total, &repair.summary);
    add(total, &repair.rerun_command_id);
    for evidence in &repair.invalidates_evidence {
        add(total, evidence);
    }
    if let Some(request) = &repair.authority_decision {
        add_request(total, request);
    }
    for ceiling in &repair.projected_ceiling_after_reverification {
        add(total, ceiling.claim_id());
        for dimension in ceiling.dimensions() {
            add(total, dimension);
        }
        for reason in ceiling.withheld_reasons() {
            add(total, reason);
        }
    }
}

fn add_request(total: &mut usize, request: &AuthorityRequest) {
    add(total, &request.target);
    add(total, &request.consequence);
    if let Some(loss) = &request.accepted_loss_required {
        add(total, loss);
    }
}

fn add_reductions(total: &mut usize, reductions: &[super::types::CeilingReduction]) {
    for reduction in reductions {
        add(total, &reduction.claim_id);
        for dimension in &reduction.dimensions {
            add(total, dimension);
        }
    }
}

fn add_scope(total: &mut usize, scope: &Scope) {
    add(total, &scope.surface);
    if let Some(path) = &scope.relative_path {
        add(total, path);
    }
}

fn add_many(total: &mut usize, values: &[String]) {
    for value in values {
        add(total, value);
    }
}

fn add(total: &mut usize, value: &str) {
    *total = total.saturating_add(value.len().saturating_add(16));
}

pub(crate) fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

pub(crate) fn valid_text(value: &str, allow_empty: bool) -> bool {
    (allow_empty || !value.is_empty())
        && value.len() <= MAX_TEXT_BYTES
        && !value.chars().any(forbidden_char)
}

pub(crate) fn valid_relative_path(value: &str) -> bool {
    valid_text(value, false)
        && !unsafe_path_value(value)
        && Path::new(value)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

pub(crate) fn valid_argv(argv: &[String]) -> bool {
    const GROUPS: &[&str] = &[
        "inspect", "next", "fit", "check", "diagnose", "prove", "observe", "package", "eval",
        "migrate",
    ];
    let group_index = if argv.get(1).is_some_and(|argument| argument == "--json") {
        2
    } else {
        1
    };
    if argv.len() <= group_index
        || argv.len() > MAX_ARGV
        || argv[0] != "ultragoal"
        || !GROUPS.contains(&argv[group_index].as_str())
    {
        return false;
    }
    argv.iter().enumerate().all(|(index, argument)| {
        let path_value = argument
            .split_once('=')
            .map_or(argument.as_str(), |(_, value)| value);
        valid_text(argument, false)
            && (argument != "--json" || index == 1)
            && !unsafe_path_value(path_value)
    })
}

pub(crate) fn proof_shaped_target(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("currentstate")
        || lower.contains("statuslabel")
        || lower
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|token| {
                matches!(
                    token,
                    "status"
                        | "label"
                        | "view"
                        | "projection"
                        | "score"
                        | "pass"
                        | "proof"
                        | "evidence"
                        | "claim"
                        | "attestation"
                        | "witness"
                        | "result"
                        | "report"
                        | "artifact"
                ) || token.starts_with("receipt")
                    || token.starts_with("telemetry")
                    || token.starts_with("generated")
            })
}

pub(crate) fn bounded_projection(bytes: Vec<u8>) -> Result<Vec<u8>, StateError> {
    if bytes.len() > MAX_PROJECTION_BYTES {
        return Err(StateError::ResourceLimit(format!(
            "projection bytes {} exceeds {}",
            bytes.len(),
            MAX_PROJECTION_BYTES
        )));
    }
    Ok(bytes)
}

fn forbidden_char(value: char) -> bool {
    value.is_control()
        || matches!(
                value,
                '\u{202a}'
                    ..='\u{202e}'
                        | '\u{2066}'
                            ..='\u{2069}'
                                | '\u{200e}'
                                | '\u{200f}'
                                | '\u{061c}'
                                | '\u{feff}'
        )
}

fn looks_like_windows_absolute(value: &str) -> bool {
    value.starts_with('\\')
        || (value.as_bytes().get(1) == Some(&b':')
            && value
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic))
}

fn unsafe_path_value(value: &str) -> bool {
    Path::new(value).is_absolute()
        || value.starts_with('~')
        || value.contains("../")
        || value.contains("..\\")
        || value == ".."
        || looks_like_windows_absolute(value)
}
