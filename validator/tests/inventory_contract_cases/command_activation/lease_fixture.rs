use std::process::Command;

fn establish_fixture_authority(repo: &TestRepo) {
    repo.commit();
    bind_fixture_lease_base(repo);
    repo.commit();
}

fn bind_fixture_lease_base(repo: &TestRepo) {
    let commit = git_output(repo, &["rev-parse", "HEAD"]);
    let tree = git_output(repo, &["rev-parse", "HEAD^{tree}"]);
    let path = repo.root.join("LANE_REGISTRY.json");
    let mut registry: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for gate in registry["prelaunch_gates"].as_array_mut().unwrap() {
        if gate["status"] == "current" {
            gate["observed_source_base"]["commit"] = commit.clone().into();
            gate["observed_source_base"]["tree"] = tree.clone().into();
        }
    }
    for record in registry["lease_state"]["active_records"]
        .as_array_mut()
        .unwrap()
    {
        record["base_commit"] = commit.clone().into();
        record["base_tree"] = tree.clone().into();
    }
    fs::write(path, serde_json::to_vec(&registry).unwrap()).unwrap();
}

fn git_output(repo: &TestRepo, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(&repo.root)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
