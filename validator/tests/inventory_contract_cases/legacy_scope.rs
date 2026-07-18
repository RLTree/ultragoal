use crate::context::LiveContext;
use crate::inventory::{AuthorityCatalog, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request};

fn catalog(repo: &TestRepo) -> AuthorityCatalog {
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    InventoryBuilder::new(&context).build().unwrap()
}

fn legacy_entry<'a>(
    catalog: &'a AuthorityCatalog,
    path: &str,
) -> Option<&'a crate::inventory::InventoryEntry> {
    catalog
        .entries()
        .iter()
        .find(|entry| entry.relative_path == path && entry.stable_id.starts_with("LEGACY-"))
}

fn assert_absent(catalog: &AuthorityCatalog, path: &str) {
    assert!(
        legacy_entry(catalog, path).is_none(),
        "unexpected legacy authority for {path}"
    );
}

fn assert_kind(catalog: &AuthorityCatalog, path: &str, kind: &str) {
    let entry = legacy_entry(catalog, path)
        .unwrap_or_else(|| panic!("missing legacy authority for {path}"));
    assert_eq!(entry.kind, format!("legacy-{kind}-authority"));
    assert!(
        entry
            .references
            .iter()
            .any(|reference| reference.starts_with("legacy-scope-evidence:")),
        "missing bounded scope evidence for {path}"
    );
}

#[test]
fn only_root_classified_test_and_history_scopes_are_excluded() {
    let repo = TestRepo::new("legacy-scope-exclusions");
    repo.write(
        "docs/ultragoal-contract-2026-07-successor-candidate-v1/00-READ-ME-FIRST.md",
        b"# Candidate\n```text\nAuthority: non-operative until explicitly adopted\n```\n<!-- Authority: non-operative until explicitly adopted -->\nAuthority: non-operative until explicitly adopted\n",
    );
    repo.write(
        "docs/ultragoal-contract-2026-07-successor-candidate-v1/contract.md",
        b"candidate contract gpt-5.5\n",
    );
    repo.write(
        "docs/ultragoal-contract-2026-07-successor-candidate-v1/CONTRACT_MANIFEST.json",
        br#"{"status":"candidate","authority":{"binding":false}}"#,
    );
    repo.write(
        "validator/tests/fixed_model.rs",
        b"const MODEL: &str = \"gpt-5.5\";\n",
    );
    repo.write(
        "validator/src/self_tests/audit/final_packet.rs",
        b"const MODEL: &str = \"gpt-5.5\";\n",
    );
    repo.write(
        "state/codex-review-artifacts/review.txt",
        b"model: gpt-5.5\n",
    );
    repo.write(
        "validator/src/inventory/legacy.rs",
        b"const DETECTOR: &str = \"gpt-5.5\";\n",
    );
    repo.write(
        "validator/src/audit/final_packet/observability/mod.rs",
        b"#[cfg(test)]\nmod tests;\npub fn active() {}\n",
    );
    repo.write(
        "validator/src/audit/final_packet/observability/tests.rs",
        b"const MODEL: &str = \"gpt-5.5\";\n",
    );
    repo.commit();

    let catalog = catalog(&repo);
    for path in [
        "validator/tests/fixed_model.rs",
        "validator/src/self_tests/audit/final_packet.rs",
        "state/codex-review-artifacts/review.txt",
        "validator/src/inventory/legacy.rs",
    ] {
        assert_absent(&catalog, path);
    }
    for path in [
        "docs/ultragoal-contract-2026-07-successor-candidate-v1/00-READ-ME-FIRST.md",
        "docs/ultragoal-contract-2026-07-successor-candidate-v1/contract.md",
        "docs/ultragoal-contract-2026-07-successor-candidate-v1/CONTRACT_MANIFEST.json",
    ] {
        assert_kind(&catalog, path, "contract");
    }
    assert_kind(
        &catalog,
        "validator/src/audit/final_packet/observability/mod.rs",
        "finalizer",
    );
    assert_kind(
        &catalog,
        "validator/src/audit/final_packet/observability/tests.rs",
        "finalizer",
    );
}

#[test]
fn active_authority_negative_controls_remain_flagged() {
    let repo = TestRepo::new("legacy-active-controls");
    for (path, content) in [
        (
            "docs/ultragoal-contract-2026-07/active.md",
            "binding predecessor contract\n",
        ),
        (
            "validator/src/argument_parser/mod.rs",
            "pub fn parse() {}\n",
        ),
        ("validator/src/command/mod.rs", "pub fn dispatch() {}\n"),
        (
            "validator/src/command/tests.rs",
            "pub fn production_named_tests() {}\n",
        ),
        (
            "validator/src/claim_semantics/lane/policy.rs",
            "pub fn lane_policy() {}\n",
        ),
        (
            "validator/src/audit/final_packet/mod.rs",
            "pub fn audit() {}\n",
        ),
        (
            "validator/src/cli/final_packet/mod.rs",
            "pub fn print() {}\n",
        ),
        ("plugin-manifest-draft.json", "{}\n"),
        ("agents/current.md", "active agent prompt\n"),
        ("docs/public-model.md", "required model gpt-5.5\n"),
        ("templates/AGENT.md", "required model gpt-5.5\n"),
    ] {
        repo.write(path, content.as_bytes());
    }
    repo.commit();
    let catalog = catalog(&repo);

    for (path, kind) in [
        ("docs/ultragoal-contract-2026-07/active.md", "contract"),
        ("validator/src/argument_parser/mod.rs", "command"),
        ("validator/src/command/mod.rs", "command"),
        ("validator/src/command/tests.rs", "command"),
        ("validator/src/claim_semantics/lane/policy.rs", "lane"),
        ("validator/src/audit/final_packet/mod.rs", "finalizer"),
        ("validator/src/cli/final_packet/mod.rs", "finalizer"),
        ("plugin-manifest-draft.json", "manifest-projection"),
        ("agents/current.md", "agent"),
        ("docs/public-model.md", "model"),
        ("templates/AGENT.md", "model"),
    ] {
        assert_kind(&catalog, path, kind);
    }
    assert_absent(&catalog, "LANE_REGISTRY.json");
    assert_absent(&catalog, "templates/LANE_REGISTRY.json");
}

#[test]
fn prose_comments_and_strings_cannot_self_declare_context_or_cfg_test_scope() {
    let repo = TestRepo::new("legacy-scope-canaries");
    repo.write(
        "notes/self-opt-out.md",
        b"Authority: non-operative until explicitly adopted\nmodel gpt-5.5\n",
    );
    repo.write(
        "docs/ultragoal-contract-2026-07-successor-candidate-v1/claim.md",
        b"Authority: non-operative until explicitly adopted\n",
    );
    repo.write(
        "validator/src/command/mod.rs",
        br##"const FAKE: &str = r#"
#[cfg(test)]
mod tests;
"#;
/*
#[cfg(test)]
mod tests;
*/
pub fn dispatch() {}
"##,
    );
    repo.write(
        "validator/src/command/tests.rs",
        b"pub fn still_production() {}\n",
    );
    repo.commit();

    let catalog = catalog(&repo);
    assert_kind(&catalog, "notes/self-opt-out.md", "model");
    assert_kind(
        &catalog,
        "docs/ultragoal-contract-2026-07-successor-candidate-v1/claim.md",
        "contract",
    );
    assert_kind(&catalog, "validator/src/command/tests.rs", "command");
}

#[test]
fn aliases_includes_and_macro_bait_keep_tests_files_active() {
    let repo = TestRepo::new("legacy-scope-module-alias-canaries");
    for (directory, declaration) in [
        (
            "path_alias",
            "#[cfg(test)]\nmod tests;\n#[path = \"tests.rs\"]\nmod production;\n",
        ),
        (
            "include_alias",
            "#[cfg(test)]\nmod tests;\ninclude!(\"tests.rs\");\n",
        ),
        (
            "macro_bait",
            "macro_rules! bait { () => { #[cfg(test)] mod tests; } }\nbait!();\n",
        ),
    ] {
        repo.write(
            &format!("validator/src/command/{directory}/mod.rs"),
            declaration.as_bytes(),
        );
        repo.write(
            &format!("validator/src/command/{directory}/tests.rs"),
            b"pub fn reachable_authority() {}\n",
        );
    }
    repo.commit();

    let catalog = catalog(&repo);
    for path in [
        "validator/src/command/path_alias/tests.rs",
        "validator/src/command/include_alias/tests.rs",
        "validator/src/command/macro_bait/tests.rs",
    ] {
        assert_kind(&catalog, path, "command");
    }
}
