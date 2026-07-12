use super::support::{CASES, READER_PROOF, catalog, catalog_result, entry, prepare};
use crate::inventory::{ActiveStatus, AuthorityState};
use crate::repository_fixture::TestRepo;
use serde_json::{Value, json};
use std::fs;

fn assert_catalog_blocks_route(repo: &TestRepo) {
    let catalog = catalog(repo);
    let legacy = entry(&catalog, CASES[0]);
    assert_eq!(legacy.authority_state, AuthorityState::Legacy);
    assert_eq!(legacy.active_status, ActiveStatus::Active);
    assert!(catalog.findings().iter().any(|finding| {
        finding.code == "invalid_agent_route_transition"
            && finding.entry_id.as_deref() == Some(legacy.stable_id.as_str())
    }));
    assert!(catalog.findings().iter().any(|finding| {
        finding.code == "parallel_authority"
            && finding.entry_id.as_deref() == Some(legacy.stable_id.as_str())
    }));
}

fn assert_reader_drift_blocks_route(repo: &TestRepo) {
    repo.commit();
    assert_catalog_blocks_route(repo);
}

fn assert_non_reader_allows_route(repo: &TestRepo) {
    repo.commit();
    let catalog = catalog(repo);
    let legacy = entry(&catalog, CASES[0]);
    assert_eq!(legacy.authority_state, AuthorityState::Context);
    assert_eq!(legacy.active_status, ActiveStatus::ContextOnly);
    assert!(!catalog.findings().iter().any(|finding| {
        finding.entry_id.as_deref() == Some(legacy.stable_id.as_str())
            && matches!(
                finding.code.as_str(),
                "invalid_agent_route_transition" | "parallel_authority"
            )
    }));
}

fn manifest(repo: &TestRepo) -> Value {
    serde_json::from_slice(&fs::read(repo.root.join("plugin-manifest-draft.json")).unwrap())
        .unwrap()
}

fn write_manifest(repo: &TestRepo, value: &Value) {
    repo.write(
        "plugin-manifest-draft.json",
        &serde_json::to_vec(value).unwrap(),
    );
}

#[test]
fn manifest_and_resource_legacy_paths_cannot_reuse_phase_a_receipt() {
    for mutation in ["agent-row", "resource-row"] {
        let repo = TestRepo::new(&format!("agent-route-{mutation}"));
        prepare(&repo, &[CASES[0]], true);
        let receipt = fs::read(repo.root.join(READER_PROOF)).unwrap();
        let mut value = manifest(&repo);
        match mutation {
            "agent-row" => {
                value["agents"][0]["path"] = json!("agents/claim-falsifier.toml");
            }
            _ => value["resources"]
                .as_array_mut()
                .unwrap()
                .push(json!("custom-agents/smuggled.toml")),
        }
        write_manifest(&repo, &value);
        assert_eq!(fs::read(repo.root.join(READER_PROOF)).unwrap(), receipt);
        assert_reader_drift_blocks_route(&repo);
    }
}

#[test]
fn compatibility_and_non_rust_positive_readers_are_not_implicit_allowlist() {
    for mutation in [
        "compatibility-rust",
        "harness-javascript",
        "extensionless-script",
    ] {
        let repo = TestRepo::new(&format!("agent-route-{mutation}"));
        prepare(&repo, &[CASES[0]], true);
        match mutation {
            "compatibility-rust" => repo.write(
                "validator/src/inventory/compatibility/unbound_reader.rs",
                br#"fn read() { let _ = std::fs::read(".codex/".to_owned() + "agents/repo-recon.toml"); }"#,
            ),
            "harness-javascript" => repo.write(
                ".harness/unbound-agent-reader.js",
                br#"const path = "custom-" + "agents/reintroduced.toml"; readFileSync(path);"#,
            ),
            _ => repo.write(
                "scripts/unbound-agent-reader",
                br#"read(".codex/" + "agents/repo-recon.toml")"#,
            ),
        }
        assert_reader_drift_blocks_route(&repo);
    }
}

#[test]
fn constructed_agent_paths_cannot_bypass_the_reader_guard() {
    for (mutation, path, source) in [
        (
            "rust-read-to-string",
            "validator/src/constructed_reader.rs",
            br#"std::fs::read_to_string(PathBuf::from([".co","dex"].concat()).join(["ag","ents"].concat()).join(role))"#.as_slice(),
        ),
        (
            "node-read-file",
            ".harness/constructed-reader.js",
            br#"fs.promises.readFile([".codex", "agents", role + ".toml"].join("/"));"#,
        ),
        (
            "go-read-file",
            "scripts/constructed_reader.go",
            br#"os.ReadFile(strings.Join([]string{".co", "dex", "ag", "ents", role}, "/"))"#,
        ),
        (
            "java-read-all-bytes",
            "hooks/ConstructedReader.java",
            br#"Files.readAllBytes(Paths.get(".co" + "dex", "ag" + "ents", role));"#,
        ),
        (
            "reviewer-separated-unrelated",
            "scripts/reviewer-separated-reader",
            br#"const ROOT=[".co","dex"]; log("unrelated"); const PARTS=["ag","ents"]; arbitrary(ROOT.concat(PARTS));"#,
        ),
        (
            "reviewer-unicode-escape",
            "scripts/reviewer-unicode-reader",
            br#"const ROOT=".co\u0064ex"; const PARTS="agents"; arbitrary(ROOT+PARTS);"#,
        ),
        (
            "extensionless-separated-variable",
            "scripts/constructed-variable-reader",
            br#"const PARTS=["ag","ents"]; const ROOT=[".co","dex"]; arbitrary(ROOT.concat(PARTS));"#,
        ),
        (
            "extensionless-nested-variable",
            "scripts/constructed-nested-reader",
            br#"const ROOT=nested(".co", list("dex", ignored("noise"))); const PARTS=nested("ag", list("ents")); arbitrary(ROOT.concat(PARTS));"#,
        ),
        (
            "escaped-slash-dot",
            "hooks/escaped-reader.js",
            br#"const path="\u002ecodex\/ag\x65nts"; arbitrary(path);"#,
        ),
        (
            "octal-escape",
            "scripts/octal-escaped-reader",
            br#"const path="\056codex\057agents"; arbitrary(path);"#,
        ),
        (
            "rust-unicode-escape",
            "validator/src/escaped_reader.rs",
            br#"let path = "\u{2e}codex\u{2f}ag\u{65}nts"; arbitrary(path);"#,
        ),
        (
            "java-prelexical-unicode",
            "hooks/PrelexicalReader.java",
            br#"String path = \u0022\u002ecodex\u002fagents\u0022; arbitrary(path);"#,
        ),
        (
            "malformed-unicode-escape",
            "scripts/malformed-constructed-reader",
            br#"const path=".co\u{zz}ex/agents"; arbitrary(path);"#,
        ),
        (
            "unterminated-escape",
            "scripts/unterminated-constructed-reader",
            b"const path=\"ordinary\\",
        ),
        (
            "extensionless-legacy-variable",
            "scripts/constructed-legacy-reader",
            br#"opaque(["custom", "agents", role]);"#,
        ),
    ] {
        let repo = TestRepo::new(&format!("agent-route-{mutation}"));
        prepare(&repo, &[CASES[0]], true);
        repo.write(path, source);
        assert_reader_drift_blocks_route(&repo);
    }
}

#[test]
fn unrelated_literal_and_comment_controls_do_not_block_legacy_demotion() {
    for (mutation, path, source) in [
        (
            "plain-platform-and-noun",
            "scripts/non-reader-control",
            br#"const platform="codex"; const noun="agents";"#.as_slice(),
        ),
        (
            "comment-only-fragments",
            "scripts/non-reader-control",
            br#"// const ROOT=[".co","dex"]; const PARTS=["ag","ents"];"#,
        ),
        (
            "divergent-path-fragment",
            "scripts/non-reader-control",
            br#"const first=".co"; const divergent="dexterity"; const tail=["ag","ents"];"#,
        ),
        (
            "unrelated-dot",
            "scripts/non-reader-control",
            br#"const punctuation="."; const platform="codex"; const noun="agents";"#,
        ),
        (
            "production-shaped-labels",
            "scripts/non-reader-control",
            br#"const ids=["batch-fanout-custom-agent-job-worker-result-discipline", "compact-agents-routed-standards", "subagent-custom-agent-sandbox-approval-inheritance"];"#,
        ),
        (
            "rust-lifetimes-and-single-quoted-prose",
            "validator/src/non_reader_lifetimes.rs",
            br#"fn borrow<'a, 'b: 'a>(value: &'a str) -> &'b str where 'a: 'b { 'outer: loop { break 'outer; } let chars = ['a', '/', '\'', '\\']; todo!() }"#,
        ),
        (
            "rust-raw-encoded-prose",
            "validator/src/non_reader_raw.rs",
            br##"let prose = r#"\u002ecodex\u002fagents"#;"##,
        ),
    ] {
        let repo = TestRepo::new(&format!("agent-route-non-reader-{mutation}"));
        prepare(&repo, &[CASES[0]], true);
        repo.write(path, source);
        assert_non_reader_allows_route(&repo);
    }
}

#[test]
fn extra_valid_canonical_agent_descriptor_blocks_legacy_demotion() {
    let repo = TestRepo::new("agent-route-extra-canonical-agent");
    prepare(&repo, &[CASES[0]], true);
    repo.write(
        ".codex/agents/extra-reviewer.toml",
        br#"name = "extra-reviewer"
description = "A syntactically valid but unauthorized seventh descriptor."
developer_instructions = "Review only."
sandbox_mode = "read-only"
"#,
    );
    assert_reader_drift_blocks_route(&repo);
}

#[test]
fn missing_empty_and_nested_agent_entries_block_legacy_demotion() {
    for mutation in ["missing", "empty-directory", "nested-entry"] {
        let repo = TestRepo::new(&format!("agent-route-{mutation}"));
        prepare(&repo, &[CASES[0]], true);
        repo.commit();
        match mutation {
            "missing" => fs::remove_file(repo.root.join(".codex/agents/repo-recon.toml")).unwrap(),
            "empty-directory" => fs::create_dir(repo.root.join(".codex/agents/empty")).unwrap(),
            "nested-entry" => repo.write(".codex/agents/nested/extra.toml", b"name = 'extra'\n"),
            _ => unreachable!(),
        }
        assert_catalog_blocks_route(&repo);
    }
}

#[cfg(unix)]
#[test]
fn symlink_hardlink_fifo_and_socket_agent_entries_block_without_opening_specials() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;
    use std::os::unix::net::UnixListener;

    for mutation in ["symlink", "fifo"] {
        let repo = TestRepo::new(&format!("agent-route-{mutation}"));
        prepare(&repo, &[CASES[0]], true);
        repo.commit();
        match mutation {
            "symlink" => symlink(
                "repo-recon.toml",
                repo.root.join(".codex/agents/extra-link.toml"),
            )
            .unwrap(),
            "fifo" => {
                let fifo = repo.root.join(".codex/agents/special-fifo");
                let name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            }
            _ => unreachable!(),
        }
        assert_catalog_blocks_route(&repo);
    }

    let repo = TestRepo::new("agent-route-hardlink");
    prepare(&repo, &[CASES[0]], true);
    repo.commit();
    fs::hard_link(
        repo.root.join(".codex/agents/repo-recon.toml"),
        repo.root.join("outside-hardlink.toml"),
    )
    .unwrap();
    assert!(
        catalog_result(&repo)
            .unwrap_err()
            .to_string()
            .contains("hard links")
    );

    let repo = TestRepo::new("agent-route-socket");
    prepare(&repo, &[CASES[0]], true);
    repo.commit();
    let short = std::path::PathBuf::from(format!("/tmp/uga-socket-{}", std::process::id()));
    let _ = fs::remove_file(&short);
    symlink(repo.root.join(".codex/agents"), &short).unwrap();
    let listener = UnixListener::bind(short.join("s")).unwrap();
    assert_catalog_blocks_route(&repo);
    drop(listener);
    fs::remove_file(short).unwrap();
}

#[test]
fn changed_allowlist_or_generic_package_consumer_blocks_route() {
    for mutation in ["allowlist", "generic-consumer"] {
        let repo = TestRepo::new(&format!("agent-route-{mutation}"));
        prepare(&repo, &[CASES[0]], true);
        let path = match mutation {
            "allowlist" => "validator/src/inventory/agent_reader_guard_digests.rs",
            _ => "validator/src/package/inventory/mod.rs",
        };
        let mut bytes = fs::read(repo.root.join(path)).unwrap();
        bytes.extend_from_slice(b"\n// drift\n");
        repo.write(path, &bytes);
        assert_reader_drift_blocks_route(&repo);
    }
}
