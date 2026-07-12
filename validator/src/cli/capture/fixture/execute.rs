use super::super::output::{self, OutputBudget};
use super::permit::FixtureCaptureAdapter;
use crate::fixture_scheduler::{
    ExpectedOutcome, FixtureExecutor, FixtureScheduleError, FixtureSpec, IsolationLease,
    ObservedOutcome, OutcomeVerdict,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

impl FixtureExecutor for FixtureCaptureAdapter {
    fn execute(
        &self,
        fixture: &FixtureSpec,
        lease: &IsolationLease,
        environment: &BTreeMap<String, String>,
    ) -> Result<ObservedOutcome, FixtureScheduleError> {
        if fixture.id != self.fixture_id || fixture.metadata_digest != self.fixture_digest {
            return Err(FixtureScheduleError::Integrity(
                "fixture execution permit does not bind this immutable specification".to_owned(),
            ));
        }
        self.validate_executable()?;
        let (exit, stdout, overflow) = run_confined(
            fixture,
            &self.executable,
            &self.arguments,
            lease.root(),
            environment,
            self.output_limit,
            &self.interrupt,
        )?;
        let expected = &fixture.expected;
        let valid_exit = matches!(expected.verdict, OutcomeVerdict::Pass) == (exit == Some(0));
        if overflow || !valid_exit || stdout != self.required_output {
            return Ok(ObservedOutcome::failure("fixture-observation-mismatch", 0));
        }
        Ok(observed_from(expected))
    }
}

fn observed_from(expected: &ExpectedOutcome) -> ObservedOutcome {
    match expected.verdict {
        OutcomeVerdict::Pass => ObservedOutcome::pass(expected.maximum_claim_ceiling),
        OutcomeVerdict::Fail => {
            ObservedOutcome::failure(&expected.causal_code, expected.maximum_claim_ceiling)
        }
        OutcomeVerdict::Quarantined => ObservedOutcome::quarantined(&expected.causal_code),
    }
}

fn run_confined(
    fixture: &FixtureSpec,
    executable: &Path,
    arguments: &[std::ffi::OsString],
    cwd: &Path,
    environment: &BTreeMap<String, String>,
    output_limit: usize,
    interrupt: &Arc<AtomicBool>,
) -> Result<(Option<i32>, Vec<u8>, bool), FixtureScheduleError> {
    #[cfg(not(unix))]
    {
        let _ = (
            executable,
            arguments,
            cwd,
            environment,
            output_limit,
            interrupt,
        );
        return Err(FixtureScheduleError::Integrity(
            "fixture execution requires Unix process groups".to_owned(),
        ));
    }
    #[cfg(unix)]
    {
        let budget = Arc::new(OutputBudget::for_sensitivity(
            output_limit,
            super::super::environment::InvocationSensitivity::Public,
        ));
        let plan = crate::fixture_scheduler::ConfinementPlan::prepare(&fixture.confinement, cwd)?;
        let mut command = plan.command(executable, arguments, cwd, environment)?;
        let started = Instant::now();
        let mut child = command.spawn().map_err(FixtureScheduleError::Io)?;
        let stdout = child.stdout.take().ok_or_else(|| {
            FixtureScheduleError::Integrity("fixture stdout unavailable".to_owned())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            FixtureScheduleError::Integrity("fixture stderr unavailable".to_owned())
        })?;
        let left = Arc::clone(&budget);
        let right = Arc::clone(&budget);
        let reader = std::thread::spawn(move || output::observe(stdout, output_limit, &left));
        let err_reader = std::thread::spawn(move || output::observe(stderr, output_limit, &right));
        let status = loop {
            if interrupt.load(Ordering::SeqCst)
                || budget.exceeded()
                || started.elapsed() >= fixture.confinement.wall_time()
            {
                terminate_and_reap(&mut child)?;
                break None;
            }
            match child.try_wait().map_err(FixtureScheduleError::Io)? {
                Some(status) => break status.code(),
                None => std::thread::sleep(Duration::from_millis(2)),
            }
        };
        let out = reader
            .join()
            .map_err(|_| {
                FixtureScheduleError::Integrity("fixture stdout reader panicked".to_owned())
            })?
            .map_err(FixtureScheduleError::Integrity)?;
        let err = err_reader
            .join()
            .map_err(|_| {
                FixtureScheduleError::Integrity("fixture stderr reader panicked".to_owned())
            })?
            .map_err(FixtureScheduleError::Integrity)?;
        let outputs = budget.finalize_streams(out, err);
        Ok((
            status,
            outputs.first.retained().to_vec(),
            outputs.output_limit_exceeded,
        ))
    }
}

#[cfg(unix)]
fn terminate_and_reap(child: &mut std::process::Child) -> Result<(), FixtureScheduleError> {
    let process = i32::try_from(child.id())
        .map_err(|_| FixtureScheduleError::Integrity("fixture process id is invalid".to_owned()))?;
    let group = process.checked_neg().ok_or_else(|| {
        FixtureScheduleError::Integrity("fixture process group id is invalid".to_owned())
    })?;
    signal_process_group(group, libc::SIGTERM)?;
    signal_process(process, libc::SIGTERM)?;
    let deadline = Instant::now() + Duration::from_millis(50);
    let mut reaped = false;
    while Instant::now() < deadline {
        if child
            .try_wait()
            .map_err(FixtureScheduleError::Io)?
            .is_some()
        {
            reaped = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let reap_error = if reaped {
        None
    } else {
        signal_process_group(group, libc::SIGKILL)?;
        signal_process(process, libc::SIGKILL)?;
        child.wait().err()
    };
    wait_for_process_group_absence(group)?;
    if let Some(error) = reap_error {
        return Err(FixtureScheduleError::Io(error));
    }
    Ok(())
}

#[cfg(unix)]
fn signal_process(process: i32, signal: i32) -> Result<(), FixtureScheduleError> {
    if unsafe { libc::kill(process, signal) } == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(FixtureScheduleError::Integrity(format!(
        "fixture process signal {signal} failed: {error}"
    )))
}

#[cfg(unix)]
fn signal_process_group(group: i32, signal: i32) -> Result<(), FixtureScheduleError> {
    if unsafe { libc::kill(group, signal) } == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(FixtureScheduleError::Integrity(format!(
        "fixture process group signal {signal} failed: {error}"
    )))
}

#[cfg(unix)]
fn wait_for_process_group_absence(group: i32) -> Result<(), FixtureScheduleError> {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if unsafe { libc::kill(group, 0) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                return Ok(());
            }
            if error.raw_os_error() != Some(libc::EPERM) {
                return Err(FixtureScheduleError::Integrity(format!(
                    "fixture process group absence probe failed: {error}"
                )));
            }
        }
        if Instant::now() >= deadline {
            return Err(FixtureScheduleError::Integrity(
                "fixture process group remained after SIGKILL".to_owned(),
            ));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
