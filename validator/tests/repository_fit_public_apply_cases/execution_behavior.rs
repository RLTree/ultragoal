use super::*;

#[test]
pub(crate) fn public_binary_applies_verifies_and_repeats_through_durable_authority() {
    let fixture = Fixture::new("positive");
    let (plan_sha256, _) = fixture.plan();
    let applied = fixture.apply(&plan_sha256);
    assert_eq!(applied.status.code(), Some(0), "{applied:?}");
    assert!(applied.stderr.is_empty(), "{applied:?}");
    let value: Value = serde_json::from_slice(&applied.stdout).unwrap();
    assert_eq!(value["schema_version"], "RepositoryFitProductionOutcome-v1");
    assert_eq!(value["status"], "applied");
    assert_eq!(value["effect"], "workspace_write");
    assert_eq!(
        fs::read(fixture.root.join("AGENTS.md")).unwrap(),
        fs::read(repository_root().join("templates/AGENTS.md")).unwrap()
    );
    assert_eq!(fs::read_dir(&fixture.authority).unwrap().count(), 3);

    let verified = fixture.verify_zero_write();
    assert_eq!(verified["schema_version"], "RepositoryFitVerification-v1");
    assert_eq!(verified["idempotent"], true);

    let before_root = snapshot(&fixture.root);
    let before_home = snapshot(&fixture.home);
    let before_temp = snapshot(&fixture.temp);
    let replayed = fixture.apply(&plan_sha256);
    assert_diagnostic(&replayed, 1, "successor_runtime_stale_context", &fixture);
    assert_eq!(snapshot(&fixture.root), before_root);
    assert_eq!(snapshot(&fixture.home), before_home);
    assert_eq!(snapshot(&fixture.temp), before_temp);

    let (repeat_plan_sha256, _) = fixture.plan();
    let before_repeat = snapshot(&fixture.root);
    let repeated = fixture.apply(&repeat_plan_sha256);
    assert_eq!(repeated.status.code(), Some(0), "{repeated:?}");
    assert!(repeated.stderr.is_empty(), "{repeated:?}");
    let value: Value = serde_json::from_slice(&repeated.stdout).unwrap();
    assert_eq!(value["status"], "idempotent");
    assert_eq!(snapshot(&fixture.root), before_repeat);
    assert_eq!(pending_entries(&fixture.pending).len(), 1);
    assert!(pending_entries(&fixture.pending)[0].ends_with(".lock"));
}

#[test]
pub(crate) fn public_binary_serializes_concurrent_apply_contenders() {
    let fixture = Fixture::new("concurrent");
    let (plan_sha256, _) = fixture.plan();
    let plan_path = fixture.plan_path();
    let args = [
        "--json",
        "fit",
        "apply",
        "--plan",
        plan_path.to_str().unwrap(),
        "--accept-plan",
        &plan_sha256,
    ];
    let gate = fixture.container.join("contender-gate.sh");
    let ready = fixture.container.join("contender-ready");
    let release = fixture.container.join("contender-release");
    fs::create_dir(&ready).unwrap();
    fs::write(
        &gate,
        "#!/bin/sh\n: \"${HUL_READY:?}\" \"${HUL_RELEASE:?}\"\ntouch \"$HUL_READY/$$\"\nwhile [ ! -f \"$HUL_RELEASE\" ]; do sleep 0.01; done\nexec \"$@\"\n",
    )
    .unwrap();
    fs::set_permissions(&gate, fs::Permissions::from_mode(0o700)).unwrap();
    let mut first = fixture.gated_command(&gate, &args);
    first.env("HUL_READY", &ready).env("HUL_RELEASE", &release);
    let mut second = fixture.gated_command(&gate, &args);
    second.env("HUL_READY", &ready).env("HUL_RELEASE", &release);
    let first = first.spawn().unwrap();
    let second = second.spawn().unwrap();
    wait_for_gate(&ready);
    fs::write(&release, b"release\n").unwrap();
    let first = first.wait_with_output().unwrap();
    let second = second.wait_with_output().unwrap();
    let outcomes = [first, second]
        .into_iter()
        .map(authoritative_contender_outcome)
        .collect::<Vec<_>>();
    assert!(outcomes.iter().any(|status| status == "applied"));
    assert!(
        outcomes
            .iter()
            .any(|status| status == "idempotent" || status == "contender_refused")
    );
    assert_eq!(fixture.verify_zero_write()["idempotent"], true);
    assert_eq!(pending_entries(&fixture.pending).len(), 1);
    assert!(pending_entries(&fixture.pending)[0].ends_with(".lock"));
}

fn wait_for_gate(ready: &Path) {
    for _ in 0..100 {
        if fs::read_dir(ready).unwrap().count() == 2 {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("both public apply contenders did not reach the start gate");
}

fn authoritative_contender_outcome(output: Output) -> String {
    match output.status.code() {
        Some(0) => {
            assert!(output.stderr.is_empty(), "{output:?}");
            serde_json::from_slice::<Value>(&output.stdout).unwrap()["status"]
                .as_str()
                .unwrap()
                .to_owned()
        }
        Some(3) => {
            assert!(output.stdout.is_empty(), "{output:?}");
            let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(
                diagnostic["diagnostic_id"],
                "successor_runtime_authority_required"
            );
            "contender_refused".to_owned()
        }
        _ => panic!("unexpected concurrent apply outcome: {output:?}"),
    }
}
