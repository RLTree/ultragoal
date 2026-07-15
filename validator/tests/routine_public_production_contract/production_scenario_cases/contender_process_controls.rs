use super::*;

#[test]
fn normal_exit_captures_bounded_streams() {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "printf stdout; printf stderr >&2"]);
    match run_bounded_contender(&mut command, Duration::from_secs(1)) {
        BoundedContender::Exited(output) => {
            assert_eq!(output.stdout, b"stdout");
            assert_eq!(output.stderr, b"stderr");
        }
        other => panic!("normal contender did not exit: {other:?}"),
    }
}

#[test]
fn pipe_pressure_is_drained_before_verified_termination() {
    let mut command = Command::new("/usr/bin/yes");
    let started = Instant::now();
    match run_bounded_contender(&mut command, Duration::from_millis(100)) {
        BoundedContender::TerminatedAndReaped(output) => {
            assert!(started.elapsed() < Duration::from_secs(1));
            assert_eq!(output.stderr, b"");
            assert_eq!(output.stdout.len(), MAX_CAPTURE_BYTES);
        }
        other => panic!("pipe-pressure contender was not reaped: {other:?}"),
    }
}
