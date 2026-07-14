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
