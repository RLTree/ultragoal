use crate::distribution::{DistributionErrorId as ErrorId, plan_package};
use crate::package_manifest::{add, manifest, spec_value, write_manifest, write_sources};
use crate::support::{Fixture, tree};
use serde_json::{Value, json};

#[test]
fn cross_component_root_prefixes_fail_in_both_orders_without_writes() {
    for parent in ["assets", "Assets"] {
        let fixture = Fixture::complete("package-component-prefix");
        write_sources(&fixture);
        let mut value = manifest();
        value["mcpServers"] = json!({"local":{"type":"stdio","command":format!("./{parent}")}});
        value["interface"]["composerIcon"] = json!("./assets/icon.svg");
        write_manifest(&fixture, &value);
        let mut spec = spec_value(false);
        add(&fixture, &mut spec, parent, "executable", true);
        add(&fixture, &mut spec, "assets/icon.svg", "data", false);
        assert_rejected_in_both_orders(&fixture, &spec);
    }
}

#[test]
fn skills_file_child_and_multisegment_prefixes_fail_closed() {
    for parent in ["skills/harness-ultragoal", "Skills/HARNESS-ULTRAGOAL"] {
        let fixture = Fixture::complete("package-skill-prefix");
        write_sources(&fixture);
        let mut spec = spec_value(false);
        add(&fixture, &mut spec, parent, "data", false);
        assert_rejected_in_both_orders(&fixture, &spec);
    }

    let fixture = Fixture::complete("package-multisegment-prefix");
    write_sources(&fixture);
    let mut spec = spec_value(false);
    add(&fixture, &mut spec, "assets/icons", "data", false);
    add(
        &fixture,
        &mut spec,
        "assets/icons/dark/icon.svg",
        "data",
        false,
    );
    assert_rejected_in_both_orders(&fixture, &spec);
}

#[test]
fn segment_siblings_remain_valid_and_planning_is_zero_write() {
    let fixture = Fixture::complete("package-prefix-siblings");
    write_sources(&fixture);
    let mut spec = spec_value(false);
    add(&fixture, &mut spec, "skills/a/SKILL.md", "skill", false);
    add(&fixture, &mut spec, "skills/ab/SKILL.md", "skill", false);
    add(
        &fixture,
        &mut spec,
        "skills/a/references/item.md",
        "documentation",
        false,
    );
    add(
        &fixture,
        &mut spec,
        "skills/ab/references/item.md",
        "documentation",
        false,
    );
    let before = tree(&fixture.root);
    plan_package(&fixture.root, &serde_json::to_vec(&spec).unwrap()).unwrap();
    spec["entries"].as_array_mut().unwrap().reverse();
    plan_package(&fixture.root, &serde_json::to_vec(&spec).unwrap()).unwrap();
    assert_eq!(tree(&fixture.root), before);
}

fn assert_rejected_in_both_orders(fixture: &Fixture, spec: &Value) {
    let before = tree(&fixture.root);
    let mut first = spec.clone();
    let mut second = spec.clone();
    second["entries"].as_array_mut().unwrap().reverse();
    for candidate in [&mut first, &mut second] {
        assert_eq!(
            plan_package(&fixture.root, &serde_json::to_vec(candidate).unwrap())
                .unwrap_err()
                .id(),
            ErrorId::InvalidSpec
        );
        assert_eq!(tree(&fixture.root), before);
    }
}
