use crate::target_fixtures::spec::model::TargetSpec;

pub fn specs() -> Vec<TargetSpec> {
    let mut out = Vec::new();
    crate::target_fixtures::baseline_specs::push(&mut out);
    crate::target_fixtures::observability_specs::push(&mut out);
    crate::target_fixtures::product_cohesion_specs::push(&mut out);
    out
}
