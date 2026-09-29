use super::*;

#[derive(Clone, Debug)]
pub(crate) struct ScheduledFixtureInvocation {
    pub(crate) executable: PathBuf,
    pub(crate) arguments: Vec<OsString>,
    pub(crate) output_limit: usize,
    pub(crate) required_output: Vec<u8>,
    pub(crate) artifact_name: String,
}

impl ScheduledFixtureInvocation {
    pub(crate) fn new(
        executable: PathBuf,
        arguments: Vec<OsString>,
        output_limit: usize,
        required_output: Vec<u8>,
        artifact_name: impl Into<String>,
    ) -> Self {
        Self {
            executable,
            arguments,
            output_limit,
            required_output,
            artifact_name: artifact_name.into(),
        }
    }
}

pub(crate) struct ScheduledFixtureEvaluationBridge {
    pub(crate) scheduler: FixtureScheduler,
    pub(crate) invocations: BTreeMap<String, ScheduledFixtureInvocation>,
    pub(crate) recovery_required: BTreeSet<String>,
}

impl ScheduledFixtureEvaluationBridge {
    pub(crate) fn new(
        root: impl AsRef<Path>,
        invocations: BTreeMap<String, ScheduledFixtureInvocation>,
    ) -> Self {
        Self {
            scheduler: FixtureScheduler::new(root),
            invocations,
            recovery_required: BTreeSet::new(),
        }
    }

    pub(crate) fn recovery_required(&self) -> &BTreeSet<String> {
        &self.recovery_required
    }
}
