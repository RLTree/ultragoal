const D0: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const D1: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const D2: &str = "sha256:2222222222222222222222222222222222222222222222222222222222222222";
const D3: &str = "sha256:3333333333333333333333333333333333333333333333333333333333333333";

#[derive(Default)]
struct Adapter {
    effects: Vec<LifecycleEffect>,
    restored: Vec<LifecycleState>,
    fail_at: Option<LifecycleEffect>,
    fail_restore: bool,
    observe_calls: Cell<usize>,
}

impl LifecycleEffectAdapter for Adapter {
    fn execute(&mut self, effect: LifecycleEffect, _: &LifecycleState) -> Result<(), String> {
        self.effects.push(effect);
        if self.fail_at == Some(effect) {
            Err(format!("injected-{effect:?}"))
        } else {
            Ok(())
        }
    }

    fn restore(&mut self, prior: &LifecycleState) -> Result<(), String> {
        self.restored.push(prior.clone());
        if self.fail_restore {
            Err("injected-restore".to_owned())
        } else {
            Ok(())
        }
    }

    fn observe_state(&self) -> Result<LifecycleState, String> {
        self.observe_calls.set(self.observe_calls.get() + 1);
        Err("adapter-observation-not-configured".to_owned())
    }
}

struct BlockingAdapter {
    effects: Vec<LifecycleEffect>,
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
}

impl LifecycleEffectAdapter for BlockingAdapter {
    fn execute(&mut self, effect: LifecycleEffect, _: &LifecycleState) -> Result<(), String> {
        self.effects.push(effect);
        if self.effects.len() == 1 {
            self.entered.wait();
            self.release.wait();
        }
        Ok(())
    }

    fn restore(&mut self, _: &LifecycleState) -> Result<(), String> {
        Err("blocking-adapter-restore-not-expected".to_owned())
    }

    fn observe_state(&self) -> Result<LifecycleState, String> {
        Err("blocking-adapter-observation-not-expected".to_owned())
    }
}

struct PartialFailureAdapter {
    attempted_effects: Vec<LifecycleEffect>,
    completed_effects: Vec<LifecycleEffect>,
    restored: Vec<LifecycleState>,
    current: LifecycleState,
    fail_at: LifecycleEffect,
    substitute_observed_candidate: bool,
}

impl PartialFailureAdapter {
    fn new(current: LifecycleState, fail_at: LifecycleEffect) -> Self {
        Self {
            attempted_effects: Vec::new(),
            completed_effects: Vec::new(),
            restored: Vec::new(),
            current,
            fail_at,
            substitute_observed_candidate: false,
        }
    }
}

impl LifecycleEffectAdapter for PartialFailureAdapter {
    fn execute(
        &mut self,
        effect: LifecycleEffect,
        expected_after: &LifecycleState,
    ) -> Result<(), String> {
        self.attempted_effects.push(effect);
        if effect == self.fail_at {
            self.current.recovery_required = true;
            return Err(format!("injected-{effect:?}"));
        }
        match effect {
            LifecycleEffect::InstallPackage => {
                self.current.installed = expected_after.installed.clone();
                self.current.generation = expected_after.generation;
            }
            LifecycleEffect::RefreshCache => {
                self.current.cache = expected_after.cache.clone();
                self.current.generation = expected_after.generation;
            }
            LifecycleEffect::RestorePriorAuthority => {
                self.current.installed = expected_after.installed.clone();
                self.current.cache = expected_after.cache.clone();
                self.current.generation = expected_after.generation;
            }
            LifecycleEffect::RemoveInstalledPackage => {
                self.current.installed = None;
                self.current.generation = expected_after.generation;
            }
            LifecycleEffect::RemoveCache => {
                self.current.cache = None;
                self.current.generation = expected_after.generation;
            }
            LifecycleEffect::VerifyInstalledBytes
            | LifecycleEffect::VerifyTeardown
            | LifecycleEffect::ProbeRuntime => {}
        }
        self.completed_effects.push(effect);
        Ok(())
    }

    fn restore(&mut self, prior: &LifecycleState) -> Result<(), String> {
        self.restored.push(prior.clone());
        Err("injected-restore".to_owned())
    }

    fn observe_state(&self) -> Result<LifecycleState, String> {
        let mut observed = self.current.clone();
        if self.substitute_observed_candidate {
            observed.installed.as_mut().unwrap().candidate_id = D2.to_owned();
        }
        Ok(observed)
    }
}

struct ZeroWriteRoot(PathBuf);

impl ZeroWriteRoot {
    fn new() -> Self {
        static NEXT_ROOT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "hul-plugin-read-failure-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("nested/empty")).unwrap();
        std::fs::write(root.join("nested/baseline.txt"), b"read-only baseline\n").unwrap();
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ZeroWriteRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn recursive_snapshot(root: &Path) -> Vec<(PathBuf, &'static str, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, rows: &mut Vec<(PathBuf, &'static str, Vec<u8>)>) {
        let metadata = std::fs::symlink_metadata(path).unwrap();
        let (kind, bytes) = if metadata.is_dir() {
            ("directory", Vec::new())
        } else if metadata.is_file() {
            ("file", std::fs::read(path).unwrap())
        } else {
            ("other", Vec::new())
        };
        rows.push((path.strip_prefix(root).unwrap().to_path_buf(), kind, bytes));
        if metadata.is_dir() {
            let mut entries = std::fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            entries.sort();
            for entry in entries {
                visit(root, &entry, rows);
            }
        }
    }

    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows
}
