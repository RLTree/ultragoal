use crate::context::{BuildRequest, EffectClass, LiveContext};
use crate::inventory::{ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256};
use crate::state::derive_current;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn current_authority_catalog_uses_only_current_owners_and_source_capture() {
    let live = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let root = std::env::temp_dir().join(format!(
        "ultragoal-current-inception-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q"]);
    git(
        &root,
        &["config", "user.email", "inception@example.invalid"],
    );
    git(&root, &["config", "user.name", "Current Inception"]);
    copy_current_authority_inputs(live, &root);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "fixture"]);

    let initial = context(&root);
    let projection = crate::product_inception::inspect(&initial)
        .unwrap()
        .to_json()
        .unwrap();
    let projection = String::from_utf8(projection).unwrap();
    assert!(projection.contains("\"CL-USABLE-LOOP\""));
    assert!(!projection.contains("PRODUCT_SUCCESS_BRIEF"));
    let state = derive_current(&initial).unwrap();
    assert_eq!(state.next_action().action_id, "inspect-current-usable-loop");

    fs::write(
        root.join("PRODUCT_SUCCESS_BRIEF.json"),
        b"retained v2 input",
    )
    .unwrap();
    fs::write(root.join("LANE_REGISTRY.json"), b"retained v2 input").unwrap();
    let changed_state = derive_current(&context(&root)).unwrap();
    assert_eq!(
        changed_state.next_action().action_id,
        "inspect-current-usable-loop",
        "retained v2 bytes cannot select a current action"
    );

    let compatibility_context = LiveContext::build(
        BuildRequest::new(&root)
            .with_effect(EffectClass::Read)
            .bind_non_secret_configuration(
                ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
                ADOPTED_HANDOFF_MANIFEST_SHA256,
            ),
    )
    .unwrap();
    assert!(
        derive_current(&compatibility_context).is_err(),
        "a full/adopted-handoff catalog context cannot silently become current authority"
    );

    let agent = root.join(".codex/agents/claim-falsifier.toml");
    let original = fs::read_to_string(&agent).unwrap();
    fs::write(
        &agent,
        original.replacen("claim-falsifier", "wrong-agent", 1),
    )
    .unwrap();
    assert!(
        derive_current(&context(&root)).is_err(),
        "a tampered current source descriptor must fail closed"
    );
    fs::remove_dir_all(root).unwrap();
}

fn copy_current_authority_inputs(live: &Path, root: &Path) {
    for relative in [
        "GOAL_CONTRACT.md",
        "PRODUCT_SUCCESS_CONTRACT.md",
        "docs/exec-plans/active/usable-product-milestone.md",
        ".codex-plugin/plugin.json",
    ] {
        let destination = root.join(relative);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(live.join(relative), destination).unwrap();
    }
    let agents = root.join(".codex/agents");
    fs::create_dir_all(&agents).unwrap();
    for entry in fs::read_dir(live.join(".codex/agents")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), agents.join(entry.file_name())).unwrap();
    }
}

fn context(root: &Path) -> LiveContext {
    LiveContext::build(BuildRequest::new(root).with_effect(EffectClass::Read)).unwrap()
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?} failed: {output:?}");
}
