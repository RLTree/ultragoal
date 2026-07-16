#[cfg(target_os = "macos")]
use super::super::filesystem::{OutputConfinement, PinnedExecutable, ReadConfinement, RootAnchor};
#[cfg(target_os = "macos")]
use super::super::outcome::RoutineCancellation;
#[cfg(target_os = "macos")]
use super::{PreparedProcess, ProcessObservation, ProcessTermination, prepare};
#[cfg(target_os = "macos")]
use std::collections::BTreeMap;
#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(target_os = "macos")]
static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "macos")]
pub(super) struct ProcessFixture {
    pub(super) root: PathBuf,
    pub(super) workspace: PathBuf,
}

#[cfg(target_os = "macos")]
impl ProcessFixture {
    pub(super) fn new(label: &str) -> Self {
        let parent = std::env::var_os("CODEX_WORKTREE_TMP")
            .map(PathBuf::from)
            .expect("managed worktree tmp is required");
        let root = parent.join(format!(
            "hul-routine-process-lifecycle-{label}-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        let workspace = root.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        Self { root, workspace }
    }

    fn run(
        &self,
        script: &str,
        timeout: Duration,
        budget: u64,
        cancellation: &RoutineCancellation,
    ) -> ProcessObservation {
        self.run_result(script, timeout, budget, cancellation)
            .unwrap()
    }

    pub(super) fn run_result(
        &self,
        script: &str,
        timeout: Duration,
        budget: u64,
        cancellation: &RoutineCancellation,
    ) -> Result<ProcessObservation, crate::routine_work::RoutineError> {
        let root = RootAnchor::open(&self.workspace).unwrap();
        let outputs = OutputConfinement::prepare(&root, &[], budget).unwrap();
        let reads = ReadConfinement {
            sources: Vec::new(),
        };
        let program = PinnedExecutable::open_unbound(std::path::Path::new("/bin/sh")).unwrap();
        let environment = BTreeMap::from([(
            crate::routine_work::CHILD_MODE_ENV.to_owned(),
            crate::routine_work::CHILD_MODE_VALUE.to_owned(),
        )]);
        observe_prepared(
            &program,
            &root,
            &outputs,
            &reads,
            &["sh".to_owned(), "-c".to_owned(), script.to_owned()],
            &environment,
            Vec::new(),
            budget,
            cancellation,
            timeout,
        )
    }

    pub(super) fn teardown(self) {
        fs::remove_dir_all(self.root).unwrap();
    }
}

pub(super) fn observe_prepared(
    program: &PinnedExecutable,
    root: &RootAnchor,
    outputs: &OutputConfinement,
    reads: &ReadConfinement,
    argv: &[String],
    environment: &BTreeMap<String, String>,
    framed_input: Vec<u8>,
    budget: u64,
    cancellation: &RoutineCancellation,
    timeout: Duration,
) -> Result<ProcessObservation, crate::routine_work::RoutineError> {
    match prepare(
        program,
        root,
        outputs,
        reads,
        argv,
        environment,
        framed_input,
        budget,
        cancellation,
    )? {
        PreparedProcess::Cancelled(observation) => Ok(observation),
        PreparedProcess::Suspended(process) => {
            let _ = process.identity()?;
            process.observe(program, root, outputs, timeout, cancellation)
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn timeout_reaps_the_exact_process_group() {
    let fixture = ProcessFixture::new("timeout");
    assert_eq!(
        fixture
            .run(
                "while :; do :; done",
                Duration::from_millis(25),
                1024,
                &RoutineCancellation::new(),
            )
            .termination,
        ProcessTermination::TimedOut
    );
    fixture.teardown();
}

#[cfg(target_os = "macos")]
#[test]
fn output_limit_reaps_the_exact_process_group() {
    let fixture = ProcessFixture::new("output-limit");
    let observation = fixture.run(
        "while :; do printf x; done",
        Duration::from_secs(2),
        64,
        &RoutineCancellation::new(),
    );
    assert_eq!(
        observation.termination,
        ProcessTermination::OutputLimit,
        "observed {} output bytes with stderr {}",
        observation.output_byte_length,
        observation.stderr_sha256,
    );
    fixture.teardown();
}

#[cfg(target_os = "macos")]
#[test]
fn cancellation_reaps_the_exact_process_group() {
    let fixture = ProcessFixture::new("cancellation");
    let cancellation = RoutineCancellation::new();
    let trigger = cancellation.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(25));
        trigger.cancel();
    });
    let termination = fixture
        .run(
            "while :; do :; done",
            Duration::from_secs(2),
            1024,
            &cancellation,
        )
        .termination;
    canceller.join().unwrap();
    assert_eq!(termination, ProcessTermination::Cancelled);
    fixture.teardown();
}

#[cfg(target_os = "macos")]
#[test]
fn child_inherits_no_authority_descriptors() {
    let fixture = ProcessFixture::new("descriptor-closure");
    let observation = fixture.run(
        "printf 'ready\\n'; descriptor=3; while [ \"$descriptor\" -le 64 ]; do if [ -e \"/dev/fd/$descriptor\" ]; then printf 'fd:%s\\n' \"$descriptor\"; fi; descriptor=$((descriptor + 1)); done",
        Duration::from_secs(2),
        1024,
        &RoutineCancellation::new(),
    );
    assert_eq!(observation.termination, ProcessTermination::Exited(0));
    assert_eq!(observation.stdout, b"ready\n");
    fixture.teardown();
}
