use super::*;

#[cfg(unix)]
pub(crate) fn terminate_and_reap(child: &mut std::process::Child) -> Result<(), String> {
    if signal_group(child.id(), libc::SIGTERM).is_err() {
        if child
            .try_wait()
            .map_err(|_| "capture process cleanup status failed".to_owned())?
            .is_some()
        {
            return Ok(());
        }
        child
            .kill()
            .map_err(|_| "capture process direct cleanup failed".to_owned())?;
        return child
            .wait()
            .map(|_| ())
            .map_err(|_| "capture process could not be reaped".to_owned());
    }
    let deadline = Instant::now() + Duration::from_millis(50);
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => std::thread::sleep(Duration::from_millis(2)),
            Err(_) => return Err("capture process cleanup status failed".to_owned()),
        }
    }
    if signal_group(child.id(), libc::SIGKILL).is_err() {
        let _ = child.kill();
    }
    child
        .wait()
        .map(|_| ())
        .map_err(|_| "capture process could not be reaped".to_owned())
}

#[cfg(test)]
mod tests {
    use super::super::{Termination, reconcile_termination, signal_group, terminate_and_reap};
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    use std::os::unix::process::CommandExt;

    #[test]
    fn cleanup_terminates_and_reaps_a_process_group() {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "while :; do :; done"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        unsafe {
            command.pre_exec(|| {
                if libc::setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().unwrap();
        let started = Instant::now();
        terminate_and_reap(&mut child).unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(child.try_wait().unwrap().is_some());
        assert!(signal_group(child.id(), libc::SIGKILL).is_ok());
    }

    #[test]
    fn output_overflow_reconciliation_preserves_observed_control_causes() {
        assert!(matches!(
            reconcile_termination(Termination::Exited { code: 0 }, true),
            Termination::OutputLimit
        ));
        assert!(matches!(
            reconcile_termination(
                Termination::Signaled {
                    signal: libc::SIGPIPE
                },
                true
            ),
            Termination::OutputLimit
        ));
        assert!(matches!(
            reconcile_termination(Termination::TimedOut, true),
            Termination::TimedOut
        ));
        assert!(matches!(
            reconcile_termination(Termination::Interrupted, true),
            Termination::Interrupted
        ));
        assert!(matches!(
            reconcile_termination(Termination::Exited { code: 0 }, false),
            Termination::Exited { code: 0 }
        ));
    }
}
