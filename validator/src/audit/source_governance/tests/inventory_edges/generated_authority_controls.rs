use super::SourceRoot;
const AGGREGATE: &str = "migration/generated-surface-authority.json";
const SHARD: &str = "migration/generated-surface-authority/fixture.json";
const POLICY: &str = "agent-standards/policy/fixture.json";
const AUDIT: &str = "agent-standards/audit/fixture.json";
const OUTPUT: &str = "agent-standards/enforcement.json";
#[test]
fn typed_zero_write_projection_accepts_exact_current_fixture() {
    let root = SourceRoot::new("generated-current");
    let before = super::super::fixture::snapshot(root.path());
    crate::audit::source_governance::capture(root.path()).expect("typed projection current");
    let current = std::env::current_dir().expect("current directory");
    let relative = root.path().strip_prefix(current).expect("relative fixture");
    crate::audit::source_governance::capture(relative).expect("relative typed projection current");
    assert_eq!(super::super::fixture::snapshot(root.path()), before);
}

#[cfg(unix)]
#[test]
fn recursive_zero_write_snapshot_rejects_mode_only_mutation() {
    use std::os::unix::fs::PermissionsExt;
    let root = SourceRoot::new("generated-mode-mutation");
    let path = root.path().join(OUTPUT);
    let before = super::super::fixture::snapshot(root.path());
    let before_bytes = std::fs::read(&path).expect("output bytes");
    let before_metadata = std::fs::metadata(&path).expect("output metadata");
    let before_modified = before_metadata.modified().expect("output modified time");
    let original_mode = before_metadata.permissions().mode();
    let mut permissions = before_metadata.permissions();
    permissions.set_mode(original_mode ^ 0o100);
    std::fs::set_permissions(&path, permissions).expect("mutate output mode");
    let after = super::super::fixture::snapshot(root.path());
    let after_metadata = std::fs::metadata(&path).expect("mutated output metadata");
    assert_eq!(
        std::fs::read(&path).expect("mutated output bytes"),
        before_bytes
    );
    assert_eq!(
        after_metadata.modified().expect("mutated modified time"),
        before_modified
    );
    assert_ne!(after, before, "mode-only mutation must change the snapshot");
}

#[test]
fn canonical_source_mutation_is_not_hidden_by_current_output_digest() {
    let root = SourceRoot::new("generated-source-mutation");
    let mutated = read(&root, POLICY).replace("fixture law", "mutated fixture law");
    root.write(POLICY, &mutated);
    assert_failure(&root, "generated_source_projection_output_stale");
}

#[test]
fn canonical_source_substitution_is_rejected_even_when_declared_everywhere() {
    let root = SourceRoot::new("generated-source-substitution");
    replace_authority_text(
        &root,
        "agent-standards/policy/fixture.json",
        "agent-standards/audit/fixture.json",
    );
    assert_failure(
        &root,
        "generated_source_projection_contract_invalid:agent-standards/enforcement.json:canonical_source_set_incomplete",
    );
}

#[test]
fn unsupported_generator_descriptor_fails_without_executing_its_command() {
    let root = SourceRoot::new("generated-descriptor");
    replace_authority_text(
        &root,
        "\"generator\": \"scripts/project-agent-standards\"",
        "\"generator\": \"scripts/check\"",
    );
    replace_authority_text(
        &root,
        "\"regeneration_command\": \"scripts/project-agent-standards write\"",
        "\"regeneration_command\": \"scripts/check\"",
    );
    assert_failure(&root, "generated_source_projection_descriptor_unsupported");
}

#[test]
fn malformed_canonical_descriptor_fails_closed() {
    let root = SourceRoot::new("generated-malformed-descriptor");
    root.write(
        POLICY,
        "{\"schema\":\"harness-ultragoal.agent-standards-policy-shard.v1\",\"schema\":\"duplicate\",\"rows\":[]}\n",
    );
    assert_failure(
        &root,
        "generated_source_projection_contract_invalid:agent-standards/enforcement.json:policy_json_invalid",
    );
}

#[test]
fn output_drift_still_fails_when_registry_digest_is_updated_to_match_it() {
    let root = SourceRoot::new("generated-output-drift");
    let previous = crate::digest::bytes(read(&root, OUTPUT).as_bytes()).replace("sha256:", "");
    root.write(OUTPUT, "{\"stale\":true}\n");
    let replacement = crate::digest::bytes(b"{\"stale\":true}\n").replace("sha256:", "");
    let aggregate = read(&root, AGGREGATE).replacen(&previous, &replacement, 1);
    root.write(AGGREGATE, &aggregate);
    assert_failure(
        &root,
        "generated_source_projection_output_stale:agent-standards/enforcement.json",
    );
}

#[test]
fn authority_source_and_output_changes_after_render_fail_final_revalidation() {
    for (label, relative) in [
        ("generated-registry-race", AGGREGATE),
        ("generated-source-race", POLICY),
        ("generated-output-race", OUTPUT),
    ] {
        let root = SourceRoot::new(label);
        let inventory = super::super::super::inventory::capture_once_for_test(root.path())
            .expect("initial governed inventory");
        let path = root.path().join(relative);
        let failures = super::super::super::generated::projection_failures_with_between(
            root.path(),
            &inventory.sources,
            || std::fs::write(path, b"{\"changed\":true}\n").expect("mutate governed source"),
        );
        assert!(
            failures.iter().any(|failure| failure
                == &format!("generated_source_projection_changed_during_session:{relative}")),
            "{relative}: {failures:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn same_byte_symlinked_ancestor_substitutions_fail_final_revalidation() {
    use std::os::unix::fs::symlink;
    for (label, ancestor, observed, registry_guard) in [
        ("registry", "migration", AGGREGATE, true),
        ("canonical-source", "agent-standards/policy", POLICY, false),
        ("output", "agent-standards", OUTPUT, false),
    ] {
        let root = SourceRoot::new(&format!("generated-ancestor-{label}"));
        let before = super::super::fixture::snapshot(root.path());
        let inventory = super::super::super::inventory::capture_once_for_test(root.path())
            .expect("initial governed inventory");
        let ancestor_path = root.path().join(ancestor);
        let displaced_path = root.path().join(format!("{ancestor}-same-bytes"));
        let failures = super::super::super::generated::projection_failures_with_between(
            root.path(),
            &inventory.sources,
            || {
                std::fs::rename(&ancestor_path, &displaced_path).expect("displace ancestor tree");
                symlink(&displaced_path, &ancestor_path).expect("substitute symlink ancestor");
            },
        );
        let expected = format!(
            "generated_source_projection_revalidation_failed:{observed}:authority_file_ancestor_rejected"
        );
        assert!(failures.contains(&expected), "{label}: {failures:?}");
        if registry_guard {
            assert!(
                failures.contains(
                    &"generated_source_registry_revalidation_failed:authority_file_ancestor_rejected"
                        .to_string()
                ),
                "{label}: {failures:?}"
            );
        }
        std::fs::remove_file(&ancestor_path).expect("remove substituted ancestor");
        std::fs::rename(&displaced_path, &ancestor_path).expect("restore ancestor tree");
        assert_eq!(super::super::fixture::snapshot(root.path()), before);
    }
}

#[cfg(unix)]
#[test]
fn special_leaf_substitution_fails_final_revalidation() {
    use std::process::Command;
    let root = SourceRoot::new("generated-output-special-race");
    let inventory = super::super::super::inventory::capture_once_for_test(root.path())
        .expect("initial governed inventory");
    let output = root.path().join(OUTPUT);
    let failures = super::super::super::generated::projection_failures_with_between(
        root.path(),
        &inventory.sources,
        || {
            std::fs::remove_file(&output).expect("remove output");
            assert!(
                Command::new("mkfifo")
                    .arg(&output)
                    .status()
                    .expect("mkfifo")
                    .success()
            );
        },
    );
    assert!(
        failures.contains(&format!(
            "generated_source_projection_revalidation_failed:{OUTPUT}:authority_file_regular_leaf_required"
        )),
        "{failures:?}"
    );
}

#[cfg(unix)]
#[test]
fn symlink_and_special_canonical_sources_fail_before_projection() {
    use std::os::unix::fs::symlink;
    use std::process::Command;
    let symlink_root = SourceRoot::new("generated-source-symlink");
    std::fs::remove_file(symlink_root.path().join(POLICY)).expect("remove policy");
    symlink(
        symlink_root.path().join(AUDIT),
        symlink_root.path().join(POLICY),
    )
    .expect("canonical source symlink");
    assert_failure(&symlink_root, "governed_source_symlink_rejected");
    let special_root = SourceRoot::new("generated-source-special");
    std::fs::remove_file(special_root.path().join(POLICY)).expect("remove policy");
    assert!(
        Command::new("mkfifo")
            .arg(special_root.path().join(POLICY))
            .status()
            .expect("mkfifo")
            .success()
    );
    assert_failure(&special_root, "governed_source_special_file_rejected");
}

fn assert_failure(root: &SourceRoot, expected: &str) {
    let failures = crate::audit::source_governance::capture(root.path()).expect_err(expected);
    assert!(
        failures.iter().any(|failure| failure.contains(expected)),
        "{expected}: {failures:?}"
    );
}

fn replace_authority_text(root: &SourceRoot, old: &str, new: &str) {
    for path in [AGGREGATE, SHARD] {
        let current = read(root, path);
        assert!(current.contains(old), "missing mutation target in {path}");
        root.write(path, &current.replace(old, new));
    }
}

fn read(root: &SourceRoot, relative: &str) -> String {
    std::fs::read_to_string(root.path().join(relative)).expect("fixture authority")
}
