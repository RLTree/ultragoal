use std::path::Path;
use std::process::Command;

fn establish_fixture_authority(repo: &TestRepo) {
    repo.commit();
    let alternate = repo.root.join(".git/objects/info/alternates");
    fs::create_dir_all(alternate.parent().unwrap()).unwrap();
    fs::write(
        alternate,
        format!(
            "{}\n",
            git_output_path(&live_root(), &["rev-parse", "--git-path", "objects"])
        ),
    )
    .unwrap();
    let tree = git_output(repo, &["rev-parse", "HEAD^{tree}"]);
    let base = source_base_commit();
    let authority = git_output(
        repo,
        &["commit-tree", &tree, "-p", &base, "-m", "fixture authority"],
    );
    run_git(repo, &["reset", "--hard", "-q", &authority]);
}

fn source_base_commit() -> String {
    let registry: serde_json::Value = serde_json::from_slice(
        &fs::read(live_root().join("LANE_REGISTRY.json")).expect("read lane registry"),
    )
    .expect("parse lane registry");
    registry["prelaunch_gates"]
        .as_array()
        .and_then(|gates| {
            gates.iter().find(|gate| {
                gate["status"].as_str() == Some("current")
                    && gate["source_authority_status"].as_str() == Some("current")
            })
        })
        .and_then(|gate| gate.pointer("/observed_source_base/commit"))
        .and_then(serde_json::Value::as_str)
        .expect("current source base commit")
        .to_owned()
}

fn git_output_path(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn git_output(repo: &TestRepo, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(&repo.root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn run_git(repo: &TestRepo, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(&repo.root)
        .status()
        .unwrap()
        .success());
}
