use super::*;

pub(crate) struct ScheduledFixtureEvaluationBridge {
    pub(crate) scheduler: FixtureScheduler,
    pub(crate) fixture_root: PathBuf,
    pub(crate) recovery_required: BTreeSet<String>,
}

impl ScheduledFixtureEvaluationBridge {
    pub(crate) const fn terminal_execution_supported() -> bool {
        cfg!(all(target_os = "macos", target_os = "freebsd"))
    }

    pub(crate) fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            scheduler: FixtureScheduler::new(root.join(".ultragoal-evaluation-runs")),
            fixture_root: root.join("evaluation/fixtures"),
            recovery_required: BTreeSet::new(),
        }
    }
}
