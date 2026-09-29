use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn current_public_state_routes_use_only_current_authority_without_writes() {
    let fixture = CurrentAuthorityFixture::new();
    assert_absent_retained_inputs(&fixture.root);
    let before_tree = tree(&fixture.root);
    let before_status = fixture.status();

    let summary = invoke(&fixture.root, ["--json", "inspect"]);
    assert_eq!(summary["schema_version"], "ProductStateSummary-v1");
    assert_current_action(&summary);

    let findings = invoke(&fixture.root, ["--json", "inspect", "findings"]);
    assert_eq!(findings["schema_version"], "ProductStateFindings-v1");
    assert_eq!(findings["findings"].as_array().unwrap().len(), 1);

    let claims = invoke(&fixture.root, ["--json", "inspect", "claims"]);
    assert_eq!(claims["schema_version"], "ProductStateClaims-v1");
    assert_eq!(claims["claim_ceilings"].as_array().unwrap().len(), 1);
    assert_eq!(claims["claim_ceilings"][0]["claim_id"], "CL-USABLE-LOOP");

    let next = invoke(&fixture.root, ["--json", "next"]);
    assert_eq!(next["schema_version"], "ProductStateNext-v1");
    assert_current_action(&next);

    let diagnose = invoke(&fixture.root, ["--json", "diagnose"]);
    assert_eq!(diagnose["schema_version"], "ProductStateDiagnose-v1");
    assert_current_action(&diagnose);

    assert_eq!(tree(&fixture.root), before_tree);
    assert_eq!(fixture.status(), before_status);
}

#[test]
fn current_public_state_route_fails_closed_on_current_authority_tamper() {
    let fixture = CurrentAuthorityFixture::new();
    let product = fixture.root.join("PRODUCT_SUCCESS_CONTRACT.md");
    let original = fs::read_to_string(&product).unwrap();
    fs::write(
        &product,
        format!("{original}\nHarness Ultragoal has one current product claim: `CL-USABLE-LOOP`.\n"),
    )
    .unwrap();
    let before_tree = tree(&fixture.root);
    let before_status = fixture.status();

    let ParseOutcome::Invocation(invocation) = parse_args(["--json", "inspect"]).unwrap() else {
        panic!("current summary route must parse");
    };
    let streams = execute_invocation(&fixture.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 4);
    assert!(streams.stdout.is_empty());
    let diagnostic: serde_json::Value = serde_json::from_slice(&streams.stderr).unwrap();
    assert_eq!(
        diagnostic["diagnostic_id"],
        "successor_runtime_state_unavailable"
    );
    assert_eq!(diagnostic["effect"], "read");
    assert_eq!(tree(&fixture.root), before_tree);
    assert_eq!(fixture.status(), before_status);
}

fn invoke<const N: usize>(root: &Path, arguments: [&str; N]) -> serde_json::Value {
    let ParseOutcome::Invocation(invocation) = parse_args(arguments).unwrap() else {
        panic!("current public route must parse");
    };
    let streams = execute_invocation(root, invocation).render(OutputMode::Json);
    assert!(
        matches!(streams.exit_code, 0 | 1),
        "state route should return a typed current result: {streams:?}"
    );
    assert!(streams.stderr.is_empty());
    serde_json::from_slice(&streams.stdout).unwrap()
}

fn assert_current_action(value: &serde_json::Value) {
    assert_eq!(
        value["next_action"]["action_id"],
        "inspect-current-usable-loop"
    );
    assert_eq!(
        value["next_action"]["command_id"], "fit-inspect",
        "the current route cannot fall back to a retained action"
    );
    assert!(!value.to_string().contains("LANE_REGISTRY"));
    assert!(!value.to_string().contains("PRODUCT_SUCCESS_BRIEF"));
}

fn assert_absent_retained_inputs(root: &Path) {
    for relative in [
        "PRODUCT_SUCCESS_BRIEF.json",
        "LANE_REGISTRY.json",
        "migration",
        "docs/ultragoal-contract-2026-07-successor-v2",
    ] {
        assert!(!root.join(relative).exists(), "fixture retains {relative}");
    }
}

struct CurrentAuthorityFixture {
    root: PathBuf,
}

impl CurrentAuthorityFixture {
    fn new() -> Self {
        let live = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let root = std::env::temp_dir().join(format!(
            "ultragoal-current-public-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "current@example.invalid"]);
        git(&root, &["config", "user.name", "Current Authority"]);
        copy_current_authority_inputs(live, &root);
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-qm", "current authority"]);
        Self { root }
    }

    fn status(&self) -> Vec<u8> {
        let output = Command::new("git")
            .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
            .env("GIT_OPTIONAL_LOCKS", "0")
            .current_dir(&self.root)
            .output()
            .unwrap();
        assert!(output.status.success());
        output.stdout
    }
}

impl Drop for CurrentAuthorityFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
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

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?} failed: {output:?}");
}
