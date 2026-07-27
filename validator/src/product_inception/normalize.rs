use super::model::{BriefV1, BriefV2};

pub(crate) fn v1(brief: &mut BriefV1) {
    brief.claim_ids.sort();
}

pub(crate) fn v2(brief: &mut BriefV2) {
    brief.claim_ids.sort();
    brief.public_entry_surface.forbidden_bypasses.sort();
    brief
        .protected_invariants
        .sort_by(|left, right| left.id.cmp(&right.id));
    for invariant in &mut brief.protected_invariants {
        invariant.claim_ids.sort();
        invariant.surfaces.sort();
    }
    for transition in &mut brief.first_truth_loop.positive_path {
        transition.dependency_ids.sort();
        transition.capability_ids.sort();
        transition.claim_ids.sort();
        transition.product_surfaces.sort();
    }
    brief
        .depth_triggers
        .sort_by(|left, right| left.trigger_id.cmp(&right.trigger_id));
    for trigger in &mut brief.depth_triggers {
        trigger.activation_finding_codes.sort();
    }
}
