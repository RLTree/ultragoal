use crate::context::{BuildRequest, EffectClass, LiveContext};
use crate::inventory::{
    InventoryBuilder, ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256,
};
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn adopted_inventory_rejects_schema_substitution() {
    assert_inventory_rejects("schema-substitution", |root| {
        fs::write(
            root.join("schemas/product-success-contract.schema.json"),
            b"{\"type\":\"null\"}",
        )
        .expect("substitute schema");
    });
}

#[test]
fn adopted_inventory_rejects_earlier_amendment_tamper() {
    assert_inventory_rejects("earlier-amendment-tamper", |root| {
        let path = root.join("AMENDMENTS.jsonl");
        let bytes = fs::read(&path).expect("amendment log");
        let mut rows = std::str::from_utf8(&bytes)
            .expect("UTF-8 amendments")
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        rows[0] = rows[0].replace("23:30:48Z", "23:30:49Z");
        fs::write(path, format!("{}\n", rows.join("\n"))).expect("tamper amendment");
    });
}

fn assert_inventory_rejects(label: &str, mutate: impl FnOnce(&Path)) {
    let live = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let root =
        std::env::temp_dir().join(format!("ultragoal-adopted-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture root");
    fs::write(root.join("tracked.txt"), b"tracked\n").expect("tracked file");
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "state@example.invalid"]);
    git(&root, &["config", "user.name", "Adopted State"]);
    super::registry::copy_authority_inputs(live, &root);
    mutate(&root);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "fixture"]);
    let context = LiveContext::build(
        BuildRequest::new(&root)
            .with_effect(EffectClass::Read)
            .bind_non_secret_configuration(
                ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
                ADOPTED_HANDOFF_MANIFEST_SHA256,
            ),
    )
    .expect("context");
    assert!(InventoryBuilder::new(&context).build().is_err());
    fs::remove_dir_all(root).expect("cleanup");
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?} failed: {output:?}");
}
