use std::path::{Path, PathBuf};
use std::process::Command;

fn register_n11_worktree(repo: &TestRepo, base: &str) -> (PathBuf, PathBuf) {
    let root = repo.root.join(".fixture-worktrees");
    let worktree = root.join("n11");
    run_git(
        repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "codex/test-n11",
            worktree.to_str().expect("UTF-8 worktree path"),
            base,
        ],
    );
    (
        root.canonicalize().expect("fixture worktree root"),
        worktree.canonicalize().expect("fixture worktree"),
    )
}

fn establish_fixture_authority(repo: &TestRepo) {
    repo.commit();
    let alternate = repo.root.join(".git/objects/info/alternates");
    fs::create_dir_all(alternate.parent().unwrap()).unwrap();
    fs::write(alternate, format!("{}\n", live_objects_path())).unwrap();
    let tree = git_output(repo, &["rev-parse", "HEAD^{tree}"]);
    let base = source_base_commit();
    let authority = git_output(
        repo,
        &["commit-tree", &tree, "-p", &base, "-m", "fixture authority"],
    );
    run_git(repo, &["reset", "--hard", "-q", &authority]);
}

fn live_objects_path() -> String {
    let raw = git_output_path(&live_root(), &["rev-parse", "--git-path", "objects"]);
    let path = Path::new(&raw);
    let path = if path.is_absolute() {
        PathBuf::from(path)
    } else {
        live_root().join(path)
    };
    path.canonicalize()
        .expect("Git object directory exists")
        .display()
        .to_string()
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
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(&repo.root)
            .status()
            .unwrap()
            .success()
    );
}

fn synthetic_request(repo: &TestRepo) -> crate::context::BuildRequest {
    use sha2::Digest;

    let manifest = repo
        .root
        .join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-HANDOFF-MANIFEST.sha256");
    let digest = format!("{:x}", sha2::Sha256::digest(fs::read(manifest).unwrap()));
    crate::context::BuildRequest::new(&repo.root)
        .bind_non_secret_configuration(crate::inventory::ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, digest)
}

fn close_synthetic_graph(repo: &TestRepo) {
    use sha2::Digest;

    let schema = "schemas/product-success-contract.schema.json";
    repo.write(schema, &fs::read(live_root().join(schema)).unwrap());
    let path = repo.root.join(
        "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/IMPLEMENTATION_DEPENDENCY_GRAPH.json",
    );
    let mut graph: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let n12 = graph["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["node_id"] == "N12-CLAIMS")
        .unwrap();
    n12["depends_on"]
        .as_array_mut()
        .unwrap()
        .retain(|dependency| dependency != "N11-EVAL-RESEARCH");
    let bytes = serde_json::to_vec(&graph).unwrap();
    fs::write(&path, &bytes).unwrap();
    let digest = format!("{:x}", sha2::Sha256::digest(&bytes));
    let contract_manifest = repo
        .root
        .join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CONTRACT_MANIFEST.json");
    let mut contract: serde_json::Value =
        serde_json::from_slice(&fs::read(&contract_manifest).unwrap()).unwrap();
    let row = contract["contract_entries"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["path"] == "FINAL-CONTRACT/IMPLEMENTATION_DEPENDENCY_GRAPH.json")
        .unwrap();
    row["sha256"] = digest.clone().into();
    row["bytes"] = (bytes.len() as u64).into();
    let contract_bytes = serde_json::to_vec(&contract).unwrap();
    fs::write(&contract_manifest, &contract_bytes).unwrap();
    let manifest = repo
        .root
        .join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-HANDOFF-MANIFEST.sha256");
    let entry = "  FINAL-CONTRACT/IMPLEMENTATION_DEPENDENCY_GRAPH.json";
    let contract_entry = "  FINAL-CONTRACT/CONTRACT_MANIFEST.json";
    let rewritten = fs::read_to_string(&manifest)
        .unwrap()
        .lines()
        .map(|line| {
            line.strip_suffix(entry)
                .filter(|_| line.len() == entry.len() + 64)
                .map_or_else(
                    || {
                        line.strip_suffix(contract_entry)
                            .filter(|_| line.len() == contract_entry.len() + 64)
                            .map_or_else(
                                || line.to_owned(),
                                |_| {
                                    format!(
                                        "{:x}{contract_entry}",
                                        sha2::Sha256::digest(&contract_bytes)
                                    )
                                },
                            )
                    },
                    |_| format!("{digest}{entry}"),
                )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let manifest_bytes = format!("{rewritten}\n").into_bytes();
    fs::write(manifest, &manifest_bytes).unwrap();
    let registry_path = repo.root.join("LANE_REGISTRY.json");
    let mut registry: serde_json::Value =
        serde_json::from_slice(&fs::read(&registry_path).unwrap()).unwrap();
    rewrite_graph_digests(&mut registry, &format!("sha256:{digest}"));
    registry["source_context"]["refs"]["graph"]["digest"] = format!("sha256:{digest}").into();
    registry["pre_adoption_source"]["contract_bundle_ref"]["digest"] =
        format!("sha256:{:x}", sha2::Sha256::digest(manifest_bytes)).into();
    fs::write(registry_path, serde_json::to_vec(&registry).unwrap()).unwrap();
}

fn rewrite_graph_digests(value: &mut serde_json::Value, digest: &str) {
    if value.get("path").and_then(serde_json::Value::as_str)
        == Some(
            "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/IMPLEMENTATION_DEPENDENCY_GRAPH.json",
        )
    {
        value["digest"] = digest.into();
    }
    match value {
        serde_json::Value::Array(values) => values
            .iter_mut()
            .for_each(|value| rewrite_graph_digests(value, digest)),
        serde_json::Value::Object(values) => values
            .values_mut()
            .for_each(|value| rewrite_graph_digests(value, digest)),
        _ => {}
    }
}
