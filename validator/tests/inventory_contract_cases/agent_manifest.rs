use crate::context::LiveContext;
use crate::inventory::{ActiveStatus, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request};

const AGENTS: [&str; 6] = [
    "repo-recon",
    "research-verifier",
    "product-journey-reviewer",
    "claim-falsifier",
    "security-reviewer",
    "orchestration-recovery-reviewer",
];

fn manifest(name: &str) -> String {
    format!(
        "name = \"{name}\"\ndescription = \"Read-only fixture agent.\"\ndeveloper_instructions = \"Inspect and report without edits.\"\nsandbox_mode = \"read-only\"\n"
    )
}

fn catalog(repo: &TestRepo) -> crate::inventory::AuthorityCatalog {
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    InventoryBuilder::new(&context).build().unwrap()
}

#[test]
fn exact_read_only_agent_manifests_satisfy_source_topology() {
    let repo = TestRepo::new("read-only-agents");
    for name in AGENTS {
        repo.write(
            &format!(".codex/agents/{name}.toml"),
            manifest(name).as_bytes(),
        );
    }
    repo.commit();
    let catalog = catalog(&repo);
    for name in AGENTS {
        let entry = catalog
            .entries()
            .iter()
            .find(|entry| entry.stable_id == format!("AGENT:{name}"))
            .unwrap();
        assert_eq!(entry.active_status, ActiveStatus::Active);
    }
    assert!(!catalog.findings().iter().any(|finding| {
        finding.code == "missing_required_component"
            && finding
                .entry_id
                .as_deref()
                .is_some_and(|id| id.starts_with("AGENT:"))
    }));
}

#[test]
fn malformed_or_write_capable_agent_manifest_fails_without_echo() {
    for (label, bytes) in [
        ("missing-instructions", b"name = \"repo-recon\"\ndescription = \"x\"\nsandbox_mode = \"read-only\"\n".as_slice()),
        ("write-sandbox", b"name = \"repo-recon\"\ndescription = \"x\"\ndeveloper_instructions = \"x\"\nsandbox_mode = \"workspace-write\"\n".as_slice()),
        ("unknown-key", b"name = \"repo-recon\"\ndescription = \"x\"\ndeveloper_instructions = \"x\"\nsandbox_mode = \"read-only\"\nmodel = \"SECRET_CANARY\"\n".as_slice()),
    ] {
        let repo = TestRepo::new(label);
        repo.write(".codex/agents/repo-recon.toml", bytes);
        repo.commit();
        let catalog = catalog(&repo);
        assert!(catalog.findings().iter().any(|finding| {
            finding.code == "invalid_agent_manifest"
                && finding.relative_path.as_deref()
                    == Some(".codex/agents/repo-recon.toml")
        }));
        let output = String::from_utf8(catalog.to_canonical_json().unwrap()).unwrap();
        assert!(!output.contains("SECRET_CANARY"));
    }
}
