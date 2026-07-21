use super::model::{BriefV1, BriefV2, CandidateBinding, ContractFacts};
use std::collections::BTreeSet;

pub(crate) fn validate_v1(brief: &BriefV1, facts: &ContractFacts) -> Result<(), &'static str> {
    if brief.product_success_contract_id != facts.product_contract_id
        || !unique_known(&brief.claim_ids, &facts.claim_ids)
    {
        return Err("brief_historical_contract_binding_invalid");
    }
    if !brief_texts_valid([
        &brief.target_problem,
        &brief.audience,
        &brief.job_to_be_done,
        &brief.context_of_use,
        &brief.desired_outcome,
        &brief.first_value_event,
        &brief.evidence_ladder,
        &brief.claim_ceiling,
    ]) {
        return Err("brief_text_invalid");
    }
    Ok(())
}

pub(crate) fn validate_v2(
    brief: &BriefV2,
    facts: &ContractFacts,
    candidate: &CandidateBinding,
) -> Result<(), &'static str> {
    if brief.product_success_contract_id != facts.product_contract_id
        || brief.product_success_contract_digest != facts.contract_digest
    {
        return Err("brief_contract_binding_stale");
    }
    if brief.real_work.repository_identity != candidate.repository_digest
        || brief.real_work.starting_candidate != candidate.candidate_digest
    {
        return Err("brief_candidate_binding_stale");
    }
    if matches!(
        (brief.real_work.dirty_state_expectation, candidate.dirty),
        (super::model::DirtyStateExpectation::Clean, true)
            | (super::model::DirtyStateExpectation::Dirty, false)
    ) {
        return Err("brief_dirty_state_mismatch");
    }
    if !unique_known(&brief.claim_ids, &facts.claim_ids) {
        return Err("brief_claim_unknown");
    }
    if brief.public_entry_surface.surface_id != "PS-ENTRY"
        || brief.public_entry_surface.route != "harness-ultragoal"
        || !known_surface("PS-ENTRY", facts)
        || !unique_texts(&brief.public_entry_surface.forbidden_bypasses, true)
    {
        return Err("brief_public_entry_unknown");
    }
    if brief.protected_invariants.is_empty()
        || !unique_ids(brief.protected_invariants.iter().map(|row| row.id.as_str()))
    {
        return Err("brief_invariants_missing_or_duplicate");
    }
    for invariant in &brief.protected_invariants {
        if invariant.disposition != "fail_closed"
            || !brief_texts_valid([&invariant.id])
            || !unique_known(&invariant.claim_ids, &facts.claim_ids)
            || !unique_texts(&invariant.surfaces, true)
            || !invariant
                .surfaces
                .iter()
                .all(|surface| known_surface(surface, facts))
            || !brief_texts_valid([&invariant.required_condition])
        {
            return Err("brief_invariant_binding_invalid");
        }
    }
    let path = &brief.first_truth_loop.positive_path;
    if path.is_empty()
        || path[0].order != 1
        || path
            .iter()
            .enumerate()
            .any(|(index, transition)| transition.order != index as u32 + 1)
        || !unique_ids(path.iter().map(|row| row.transition_id.as_str()))
        || !path.iter().all(|transition| {
            brief_texts_valid([&transition.transition_id, &transition.expected_observation])
                && unique_texts(&transition.dependency_ids, false)
                && unique_texts(&transition.capability_ids, false)
                && unique_known(&transition.claim_ids, &facts.claim_ids)
                && unique_texts(&transition.product_surfaces, true)
                && transition
                    .product_surfaces
                    .iter()
                    .all(|surface| known_surface(surface, facts))
                && brief_texts_valid([&transition.expected_observation])
        })
        || !path
            .iter()
            .any(|row| row.transition_id == brief.first_truth_loop.first_value_transition)
        || !path
            .iter()
            .any(|row| row.transition_id == brief.first_truth_loop.failure_control.transition_id)
    {
        return Err("brief_truth_loop_invalid");
    }
    if !unique_ids(
        brief
            .depth_triggers
            .iter()
            .map(|trigger| trigger.trigger_id.as_str()),
    ) && !brief.depth_triggers.is_empty()
    {
        return Err("brief_depth_trigger_duplicate");
    }
    if brief.depth_triggers.iter().any(|trigger| {
        !brief_texts_valid([&trigger.trigger_id])
            || !unique_texts(&trigger.activation_finding_codes, true)
            || !brief_texts_valid([
                &trigger.risk_or_claim,
                &trigger.smallest_investment,
                &trigger.fitness_function,
                &trigger.invalidation_condition,
            ])
    }) {
        return Err("brief_depth_trigger_invalid");
    }
    if !brief_texts_valid([
        &brief.target_problem,
        &brief.audience,
        &brief.job_to_be_done,
        &brief.context_of_use,
        &brief.desired_outcome,
        &brief.first_value_event,
        &brief.operator.actor_reference,
        &brief.real_work.task_id,
        &brief.real_work.task,
        &brief.real_work.expected_useful_outcome,
        &brief.first_truth_loop.loop_id,
        &brief.first_truth_loop.first_value_transition,
        &brief.first_truth_loop.preservation_expectation,
        &brief.first_truth_loop.repeat_use_expectation,
        &brief.first_truth_loop.failure_control.failure,
        &brief.first_truth_loop.failure_control.diagnosis,
        &brief.first_truth_loop.failure_control.recovery,
        &brief.first_truth_loop.failure_control.preservation,
        &brief.evidence_ladder,
        &brief.claim_ceiling,
    ]) {
        return Err("brief_text_invalid");
    }
    Ok(())
}

fn unique_known(values: &[String], known: &[String]) -> bool {
    let known = known.iter().collect::<BTreeSet<_>>();
    !values.is_empty()
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
        && values.iter().all(|value| known.contains(value))
}

fn unique_ids<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let values = values.collect::<Vec<_>>();
    !values.is_empty() && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn known_surface(surface: &str, facts: &ContractFacts) -> bool {
    facts.surface_ids.iter().any(|known| known == surface)
}

fn brief_texts_valid<'a>(values: impl IntoIterator<Item = &'a String>) -> bool {
    values.into_iter().all(|value| {
        !value.is_empty()
            && value.len() <= 4096
            && !value.chars().any(char::is_control)
            && !value.contains(".git/")
    })
}

fn unique_texts(values: &[String], nonempty: bool) -> bool {
    (!nonempty || !values.is_empty())
        && values.iter().all(|value| brief_texts_valid([value]))
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}
