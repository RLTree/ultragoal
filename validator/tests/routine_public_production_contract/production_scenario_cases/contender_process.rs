use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const CONTENDER_POLL_CADENCE: Duration = Duration::from_millis(5);

pub(crate) enum BoundedContender {
    Exited(Output),
    TimedOut {
        output: Output,
        kill_error: Option<std::io::Error>,
    },
}

pub(crate) fn run_bounded_contender(command: &mut Command, bound: Duration) -> BoundedContender {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_for_contender(child, bound)
}

fn wait_for_contender(mut child: Child, bound: Duration) -> BoundedContender {
    let deadline = Instant::now() + bound;
    loop {
        if Instant::now() >= deadline {
            let kill_error = child.kill().err();
            let output = child.wait_with_output().unwrap();
            return BoundedContender::TimedOut { output, kill_error };
        }
        if child.try_wait().unwrap().is_some() {
            return BoundedContender::Exited(child.wait_with_output().unwrap());
        }
        std::thread::sleep(CONTENDER_POLL_CADENCE);
    }
}
