use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use ultragoal::context::{BuildRequest, EffectClass, LiveContext};
use ultragoal::inventory::{
    ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256, InventoryBuilder,
};
use ultragoal::state::{
    CommandBinding, DependencyActionCatalog, DependencyActionSpec, HostGoalObservation,
    RuntimeMetadata, StateEngine, StateError,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn mutation_after_context_construction_is_rejected_at_state_boundary() {
    let repo = TempRepo::new();
    let context = LiveContext::build(BuildRequest::new(&repo.root).bind_non_secret_configuration(
        ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
        ADOPTED_HANDOFF_MANIFEST_SHA256,
    ))
    .unwrap();
    let authority = InventoryBuilder::new(&context).build().unwrap();
    let policy = DependencyActionCatalog::from_untrusted_spec(DependencyActionSpec {
        expected_context_id: context.context_id().to_owned(),
        expected_authority_catalog_id: authority.catalog_id().to_owned(),
        claims: Vec::new(),
        dependencies: Vec::new(),
        inventory_policies: Vec::new(),
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
    })
    .unwrap();
    fs::write(repo.root.join("tracked.txt"), b"mutated after context\n").unwrap();
    assert!(matches!(
        StateEngine::derive(&context, &authority, &policy),
        Err(StateError::StaleContext(_))
    ));
}

struct TempRepo {
    root: PathBuf,
}

impl TempRepo {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-state-stale-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "state@example.invalid"]);
        git(&root, &["config", "user.name", "State Test"]);
        let live = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
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
        for name in ["authority-routes.json", "generated-surface-authority.json"] {
            fs::copy(
                live.join("migration").join(name),
                root.join("migration").join(name),
            )
            .unwrap();
        }
        fs::create_dir_all(root.join(".codex-plugin")).unwrap();
        fs::write(
            root.join(".codex-plugin/plugin.json"),
            br#"{"name":"harness-ultragoal","version":"0.0.0-test"}"#,
        )
        .unwrap();
        fs::write(root.join("tracked.txt"), b"initial\n").unwrap();
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-q", "-m", "fixture"]);
        Self { root }
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
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
