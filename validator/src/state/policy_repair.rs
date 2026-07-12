use super::limits::{proof_shaped_target, valid_id, valid_text};
use super::types::{AuthorityRequirement, Repair};
use crate::context::EffectClass;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn register<'a>(
    repair: &'a Repair,
    repairs: &mut BTreeMap<&'a str, &'a Repair>,
    claims: &BTreeMap<&str, BTreeSet<&str>>,
    problems: &mut Vec<String>,
) {
    if !valid_id(&repair.repair_id)
        || !valid_id(&repair.target.id)
        || !valid_text(&repair.summary, false)
        || !valid_id(&repair.rerun_command_id)
        || repair.invalidates_evidence.is_empty()
        || repair
            .invalidates_evidence
            .iter()
            .any(|item| !valid_id(item))
    {
        problems.push("invalid-repair-shape".to_owned());
    }
    if proof_shaped_target(&repair.target.id) {
        problems.push("proof-shaped-repair-target".to_owned());
    }
    validate_effect_authority(repair, problems);
    validate_decision(repair, problems);
    if let Some(previous) = repairs.insert(&repair.repair_id, repair)
        && previous != repair
    {
        problems.push("conflicting-repair-id".to_owned());
    }
    for ceiling in &repair.projected_ceiling_after_reverification {
        let Some(maximum) = claims.get(ceiling.claim_id()) else {
            problems.push("repair-ceiling-unknown-claim".to_owned());
            continue;
        };
        if ceiling
            .dimensions()
            .iter()
            .any(|dimension| !maximum.contains(dimension.as_str()))
        {
            problems.push("repair-ceiling-exceeds-maximum".to_owned());
        }
    }
}

fn validate_effect_authority(repair: &Repair, problems: &mut Vec<String>) {
    let invalid = match repair.effect {
        EffectClass::Read | EffectClass::PlannedWrite => false,
        EffectClass::WorkspaceWrite => !matches!(
            repair.authority,
            AuthorityRequirement::Workspace | AuthorityRequirement::Root
        ),
        EffectClass::ExternalWrite => !matches!(
            repair.authority,
            AuthorityRequirement::External | AuthorityRequirement::HumanDestructive
        ),
        EffectClass::Destructive => repair.authority != AuthorityRequirement::HumanDestructive,
    };
    if invalid {
        problems.push("repair-effect-authority-mismatch".to_owned());
    }
}

fn validate_decision(repair: &Repair, problems: &mut Vec<String>) {
    let requires = matches!(
        repair.authority,
        AuthorityRequirement::External | AuthorityRequirement::HumanDestructive
    );
    if (requires && repair.authority_decision.is_none())
        || (repair.authority == AuthorityRequirement::None && repair.authority_decision.is_some())
    {
        problems.push("repair-authority-decision-mismatch".to_owned());
        return;
    }
    let Some(decision) = &repair.authority_decision else {
        return;
    };
    if !valid_id(&decision.target)
        || proof_shaped_target(&decision.target)
        || !valid_text(&decision.consequence, false)
        || decision
            .accepted_loss_required
            .as_deref()
            .is_some_and(|value| !valid_text(value, false))
    {
        problems.push("invalid-authority-decision".to_owned());
    }
    if repair.authority == AuthorityRequirement::HumanDestructive
        && decision
            .accepted_loss_required
            .as_deref()
            .is_none_or(str::is_empty)
    {
        problems.push("destructive-loss-not-named".to_owned());
    }
}
