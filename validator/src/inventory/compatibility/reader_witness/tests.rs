use super::scan::looks_like_agent_reader_at;
use super::*;
use crate::context::{BuildRequest, LiveContext};
use crate::inventory::{ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256};

#[test]
fn live_reader_guard_has_no_unbound_positive_reader() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let request = BuildRequest::new(root).bind_non_secret_configuration(
        ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
        ADOPTED_HANDOFF_MANIFEST_SHA256,
    );
    let context = LiveContext::build(request).unwrap();
    let reads = context.begin_read_session().unwrap();
    assert!(
        phase_a_context_intact(&reads, root),
        "Phase A context receipt drift"
    );
    assert_eq!(
        read_bounded(&reads, &root.join(GUARD_PATH), MAX_SOURCE_BYTES).unwrap(),
        GUARD_BYTES,
        "compiled guard source drift"
    );
    let stale = all_readers()
        .filter(|spec| !reader_current(&reads, root, spec))
        .map(|spec| spec.path)
        .collect::<Vec<_>>();
    assert!(stale.is_empty(), "compiled reader drift: {stale:?}");
    assert!(
        exact_manifest_state(&reads, root),
        "canonical six-agent manifest drift"
    );
    let unbound = unbound_reader_paths(&reads, root).unwrap();
    assert!(unbound.is_empty(), "unbound positive readers: {unbound:?}");
}

#[test]
fn constructed_agent_paths_are_detected_without_treating_manifests_as_readers() {
    for source in [
        br#"std::fs::read_to_string(PathBuf::from([".co","dex"].concat()).join(["ag","ents"].concat()).join(role))"#.as_slice(),
        br#"fs.promises.readFile([".codex", "agents", role + ".toml"].join("/"));"#,
        br#"os.ReadFile(strings.Join([]string{".co", "dex", "ag", "ents", role}, "/"))"#,
        br#"Files.readAllBytes(Paths.get(".co" + "dex", "ag" + "ents", role));"#,
        br#"const ROOT=[".co","dex"]; log("unrelated"); const PARTS=["ag","ents"]; arbitrary(ROOT.concat(PARTS));"#,
        br#"const ROOT=".co\u0064ex"; const PARTS="agents"; arbitrary(ROOT+PARTS);"#,
        br#"const PARTS=["ag","ents"]; const ROOT=[".co","dex"]; consume(ROOT.concat(PARTS));"#,
        br#"const ROOT=nested(".co", list("dex", ignored("noise"))); const PARTS=nested("ag", list("ents")); arbitrary(ROOT.concat(PARTS));"#,
        br#"const ROOT=[".co","dex"]; const PARTS="agents/role.toml"; arbitrary(ROOT.concat(PARTS));"#,
        br#"const path="\u002ecodex\/ag\x65nts"; arbitrary(path);"#,
        br#"const path="\x2e\x63\x6f\x64\x65\x78\x2f\x61\x67\x65\x6e\x74\x73"; arbitrary(path);"#,
        br#"const path="\056codex\057agents"; arbitrary(path);"#,
        br#"let path = "\u{2e}codex\u{2f}ag\u{65}nts"; arbitrary(path);"#,
        br#"let path = "\U0000002ecodex\x2fagents"; arbitrary(path);"#,
        br#"String path = \u0022\u002ecodex\u002fagents\u0022; arbitrary(path);"#,
        br#"String path = \uuuu0022\u002ecodex\u002fagents\uu0022; arbitrary(path);"#,
        br#"const path=".co\u{zz}ex/agents"; arbitrary(path);"#,
        br#"const malformed="\x2g";"#,
        br#"const malformed="\u{110000}";"#,
        b"const path=\".codex/agents\\".as_slice(),
        b"const path=\"ordinary\\".as_slice(),
        br####"let text = r###"ordinary"##;"####,
        br#"const path='.codex/agents'; arbitrary(path);"#,
        br#"opaque(["custom", "agents", role]);"#,
    ] {
        assert!(looks_like_agent_reader(source), "missed {source:?}");
    }
    for source in [
        br#"const platform="codex"; const noun="agents";"#.as_slice(),
        br#"// const ROOT=[".co","dex"]; const PARTS=["ag","ents"];"#,
        br#"const first=".co"; const divergent="dexterity"; const tail=["ag","ents"];"#,
        br#"const root=".codex"; const unrelated="agentship";"#,
        br#"const root="codex."; const unrelated="agents";"#,
        br#"const punctuation="."; const platform="codex"; const noun="agents";"#,
        br#"const escaped="\u0063olumn\/value"; const noun="agents";"#,
        br#"const ids=["batch-fanout-custom-agent-job-worker-result-discipline", "compact-agents-routed-standards", "subagent-custom-agent-sandbox-approval-inheritance"];"#,
        br#"const messages=["a product command is required", "group lookup failed", "error text", "no transition selected"];"#,
        br#"fn borrow<'a, 'b: 'a>(value: &'a str) -> &'b str where 'a: 'b { 'outer: loop { break 'outer; } let chars = ['a', '/', '\'', '\\']; todo!() }"#,
        br####"let prose = r###"codex and agents are unrelated labels"###;"####,
        br#"struct Holder<'scope> { value: &'scope str } fn f(v: Holder<'_>) { let label='ordinary prose'; }"#,
    ] {
        assert!(
            !looks_like_agent_reader(source),
            "false positive for {source:?}"
        );
    }
    assert!(looks_like_agent_reader_at(
        "validator/src/raw_reader.rs",
        br##"let path = r#".codex/agents/role.toml"#;"##,
    ));
    assert!(!looks_like_agent_reader_at(
        "validator/src/raw_prose.rs",
        br##"let prose = r#"\u002ecodex\u002fagents"#;"##,
    ));
    for manifest in MANIFESTS {
        assert!(
            !looks_like_agent_reader(manifest.bytes),
            "canonical descriptor misclassified: {}",
            manifest.path
        );
    }
}
