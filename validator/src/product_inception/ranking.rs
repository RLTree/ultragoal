use super::model::{DepthTriggerKind, InceptionEffect};
use super::parser::ParsedBrief;
use super::{InceptionError, normalize, parser, reader, validation};
use crate::context::{EffectClass, LiveContext};
use crate::inventory::AuthorityCatalog;
use crate::state::{ActionDefinition, ActionPriorityClass, EvidenceLedActionBinding};
use std::collections::BTreeSet;

pub(crate) enum RankingDisposition {
    Active,
    InceptionRequired,
}

pub(crate) fn bind_actions(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    actions: &mut [ActionDefinition],
    current_findings: &BTreeSet<String>,
) -> RankingDisposition {
    bind(context, catalog, actions, current_findings)
        .unwrap_or(RankingDisposition::InceptionRequired)
}

fn bind(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    actions: &mut [ActionDefinition],
    current_findings: &BTreeSet<String>,
) -> Result<RankingDisposition, InceptionError> {
    let input = reader::read_with_catalog(context, catalog.clone())?;
    let Some(bytes) = input.brief else {
        return Ok(RankingDisposition::InceptionRequired);
    };
    let ParsedBrief::EvidenceLed(mut brief) = parser::parse(&bytes)? else {
        return Ok(RankingDisposition::InceptionRequired);
    };
    normalize::v2(&mut brief);
    validation::validate_v2(&brief, &input.facts, &input.candidate)
        .map_err(InceptionError::Code)?;
    let brief_digest = crate::digest::bytes(&bytes);
    bind_validated(&brief, &brief_digest, actions, current_findings)
}

pub(super) fn bind_validated(
    brief: &super::model::BriefV2,
    brief_digest: &str,
    actions: &mut [ActionDefinition],
    current_findings: &BTreeSet<String>,
) -> Result<RankingDisposition, InceptionError> {
    let active = brief
        .depth_triggers
        .iter()
        .filter(|trigger| {
            trigger
                .activation_finding_codes
                .iter()
                .any(|code| current_findings.contains(code))
        })
        .map(|trigger| trigger.trigger_id.clone())
        .collect::<Vec<_>>();
    let parked = brief
        .depth_triggers
        .iter()
        .filter(|trigger| !active.contains(&trigger.trigger_id))
        .map(|trigger| trigger.trigger_id.clone())
        .collect::<Vec<_>>();
    for transition in &brief.first_truth_loop.positive_path {
        let action = exact_action(actions, &transition.action_id)?;
        if action.command_id.as_deref() != Some(transition.command_id.as_str())
            || action.effect != effect(transition.effect)
            || !same(&action.requires_dependencies, &transition.dependency_ids)
            || !same(&action.required_capabilities, &transition.capability_ids)
        {
            return Ok(RankingDisposition::InceptionRequired);
        }
        action.evidence_led = Some(binding(
            ActionPriorityClass::ActiveTruthLoopTransition,
            brief_digest,
            Some(&transition.transition_id),
            Some(transition.order),
            &active,
            &parked,
        ));
    }
    for trigger in &brief.depth_triggers {
        if !active.contains(&trigger.trigger_id) {
            continue;
        }
        let action = exact_action(actions, &trigger.action_id)?;
        let class = match trigger.kind {
            DepthTriggerKind::ProtectedInvariant => ActionPriorityClass::ProtectedInvariant,
            DepthTriggerKind::ObservedFailure => ActionPriorityClass::FalsePassOrRejection,
            DepthTriggerKind::RepeatedGap => ActionPriorityClass::RepeatedCrossContextGap,
            DepthTriggerKind::BoundedExperiment => ActionPriorityClass::BoundedExperiment,
        };
        if action
            .evidence_led
            .as_ref()
            .is_none_or(|current| class < current.class)
        {
            action.evidence_led = Some(binding(class, brief_digest, None, None, &active, &parked));
        }
    }
    Ok(RankingDisposition::Active)
}

fn exact_action<'a>(
    actions: &'a mut [ActionDefinition],
    action_id: &str,
) -> Result<&'a mut ActionDefinition, InceptionError> {
    let mut matches = actions
        .iter_mut()
        .filter(|action| action.action_id == action_id);
    let action = matches
        .next()
        .ok_or(InceptionError::Code("brief_action_unknown"))?;
    if matches.next().is_some() {
        return Err(InceptionError::Code("brief_action_ambiguous"));
    }
    Ok(action)
}

fn binding(
    class: ActionPriorityClass,
    brief_digest: &str,
    transition_id: Option<&str>,
    transition_order: Option<u32>,
    active: &[String],
    parked: &[String],
) -> EvidenceLedActionBinding {
    EvidenceLedActionBinding {
        class,
        brief_digest: brief_digest.to_owned(),
        transition_id: transition_id.map(ToOwned::to_owned),
        transition_order,
        active_trigger_ids: active.to_vec(),
        parked_trigger_ids: parked.to_vec(),
    }
}

fn effect(value: InceptionEffect) -> EffectClass {
    match value {
        InceptionEffect::Read => EffectClass::Read,
        InceptionEffect::PlannedWrite => EffectClass::PlannedWrite,
        InceptionEffect::WorkspaceWrite => EffectClass::WorkspaceWrite,
        InceptionEffect::ExternalWrite => EffectClass::ExternalWrite,
        InceptionEffect::Destructive => EffectClass::Destructive,
    }
}

fn same(left: &[String], right: &[String]) -> bool {
    left.iter().collect::<BTreeSet<_>>() == right.iter().collect::<BTreeSet<_>>()
}
