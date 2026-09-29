use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use ultragoal::context::{BuildRequest, EffectClass, LiveContext};
use ultragoal::inventory::{
    ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256, AuthorityCatalog,
    InventoryBuilder,
};
use ultragoal::state::{
    AuthorityRequirement, ClaimSpec, CommandBinding, DependencyActionSpec, HostGoalObservation,
    InventoryPolicy, Repair, RepairTarget, RepairTargetKind, RuntimeMetadata,
};

use super::authority_inputs;

pub(super) struct Fixture {
    pub root: PathBuf,
    pub context: LiveContext,
    pub authority: AuthorityCatalog,
    pub spec: DependencyActionSpec,
}

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) fn fixture() -> Fixture {
    let live = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let root = std::env::temp_dir().join(format!(
        "ultragoal-state-public-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "state@example.invalid"]);
    git(&root, &["config", "user.name", "State Test"]);
    copy_authority_inputs(&live, &root);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "fixture"]);
    let context = LiveContext::build(BuildRequest::new(&root).bind_non_secret_configuration(
        ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
        ADOPTED_HANDOFF_MANIFEST_SHA256,
    ))
    .unwrap();
    let authority = InventoryBuilder::new(&context).build().unwrap();
    let codes = authority
        .findings()
        .iter()
        .map(|finding| finding.code.clone())
        .collect::<BTreeSet<_>>();
    let inventory_policies = codes
        .into_iter()
        .map(|code| InventoryPolicy {
            scope_surface: "live-authority-catalog".to_owned(),
            repair: repair_for(&code),
            code,
            ceiling_reductions: vec![ultragoal::state::CeilingReduction {
                claim_id: "CL-ORCHESTRATION".to_owned(),
                dimensions: BTreeSet::from(["inventory-reconciled".to_owned()]),
            }],
        })
        .collect();
    let spec = DependencyActionSpec {
        expected_context_id: context.context_id().to_owned(),
        expected_authority_catalog_id: authority.catalog_id().to_owned(),
        claims: vec![ClaimSpec {
            claim_id: "CL-ORCHESTRATION".to_owned(),
            maximum_dimensions: vec!["inventory-reconciled".to_owned()],
        }],
        dependencies: Vec::new(),
        inventory_policies,
        capability_requirements: Vec::new(),
        runtime_metadata: RuntimeMetadata::default(),
        runtime_requirements: Vec::new(),
        commands: vec![CommandBinding {
            command_id: "inspect-json".to_owned(),
            argv: vec![
                "ultragoal".to_owned(),
                "--json".to_owned(),
                "inspect".to_owned(),
            ],
            effect: EffectClass::Read,
        }],
        actions: Vec::new(),
        host_goal: HostGoalObservation::default(),
    };
    Fixture {
        root,
        context,
        authority,
        spec,
    }
}

pub(super) fn repair_for(code: &str) -> Repair {
    Repair {
        repair_id: format!("reconcile-{code}"),
        target: RepairTarget {
            kind: RepairTargetKind::Source,
            id: "live-authority-source".to_owned(),
        },
        summary: format!("Reconcile live inventory finding {code} at its source"),
        effect: EffectClass::PlannedWrite,
        authority: AuthorityRequirement::Root,
        rerun_command_id: "inspect-json".to_owned(),
        authority_decision: None,
        invalidates_evidence: BTreeSet::from([format!("inventory-finding:{code}")]),
        projected_ceiling_after_reverification: Vec::new(),
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn copy_authority_inputs(live: &Path, root: &Path) {
    fs::copy(
        live.join("LANE_REGISTRY.json"),
        root.join("LANE_REGISTRY.json"),
    )
    .unwrap();
    fs::create_dir_all(root.join("templates")).unwrap();
    fs::copy(
        live.join("templates/LANE_REGISTRY.json"),
        root.join("templates/LANE_REGISTRY.json"),
    )
    .unwrap();
    let source = live.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
    let target = root.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
    fs::create_dir_all(&target).unwrap();
    let mut contract_files = fs::read_dir(&source)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    contract_files.sort();
    for path in contract_files {
        fs::copy(&path, target.join(path.file_name().unwrap())).unwrap();
    }
    for name in ["FINAL-HANDOFF-MANIFEST.sha256", "README.md"] {
        fs::copy(
            source.parent().unwrap().join(name),
            target.parent().unwrap().join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(root.join("migration")).unwrap();
    for name in ["authority-routes.json"] {
        fs::copy(
            live.join("migration").join(name),
            root.join("migration").join(name),
        )
        .unwrap();
    }
    authority_inputs::copy_declared_files(live, root);
    fs::create_dir_all(root.join(".codex-plugin")).unwrap();
    fs::write(
        root.join(".codex-plugin/plugin.json"),
        br#"{"name":"harness-ultragoal","version":"0.0.0-test"}"#,
    )
    .unwrap();
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
}
