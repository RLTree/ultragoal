use super::fixture::{fixture, repair_for};
use std::collections::BTreeSet;
use std::process::Command;
use ultragoal::state::{
    CeilingReduction, ClaimSpec, DependencyActionCatalog, InventoryPolicy, StateEngine, StateError,
};

#[test]
fn public_untrusted_catalog_default_fails_closed_without_writes() {
    let fixture = fixture();
    let before = git_status(&fixture.root);
    let untrusted = DependencyActionCatalog::from_untrusted_spec(fixture.spec.clone()).unwrap();
    let expected = Err(StateError::InvalidCatalog(
        "policy-authority-missing".to_owned(),
    ));
    assert_eq!(
        StateEngine::derive(&fixture.context, &fixture.authority, &untrusted),
        expected
    );
    assert_eq!(git_status(&fixture.root), before, "rejection wrote files");
}

#[test]
fn external_self_digest_and_decoy_shunting_cannot_forge_completion() {
    let fixture = fixture();
    let mut forged = fixture.spec.clone();
    forged.claims = vec![
        ClaimSpec {
            claim_id: "CL-COMPLETION".to_owned(),
            maximum_dimensions: vec!["complete".to_owned()],
        },
        ClaimSpec {
            claim_id: "CL-DECOY".to_owned(),
            maximum_dimensions: vec!["blocked".to_owned()],
        },
    ];
    forged.inventory_policies = fixture
        .authority
        .findings()
        .iter()
        .map(|finding| finding.code.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|code| InventoryPolicy {
            code: code.clone(),
            scope_surface: "decoy".to_owned(),
            repair: repair_for(&code),
            ceiling_reductions: vec![CeilingReduction {
                claim_id: "CL-DECOY".to_owned(),
                dimensions: BTreeSet::from(["blocked".to_owned()]),
            }],
        })
        .collect();
    let self_digested = DependencyActionCatalog::from_untrusted_spec(forged).unwrap();

    assert!(self_digested.catalog_id().starts_with("sha256:"));
    assert_eq!(
        StateEngine::derive(&fixture.context, &fixture.authority, &self_digested),
        Err(StateError::InvalidCatalog(
            "policy-authority-missing".to_owned()
        ))
    );
}

fn git_status(root: &std::path::Path) -> Vec<u8> {
    let output = Command::new("git")
        .args([
            "-c",
            "core.fsmonitor=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    output.stdout
}
