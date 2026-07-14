use super::*;

pub(crate) fn output_with_timeout(mut command: Command, timeout: Duration) -> Output {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let mut stderr = child.stderr.take().unwrap();
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return Output {
                status,
                stdout: stdout_reader.join().unwrap(),
                stderr: stderr_reader.join().unwrap(),
            };
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            panic!("source-built command exceeded {timeout:?}");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

pub(crate) fn safe_label(label: &str) -> String {
    label
        .chars()
        .filter(|value| value.is_ascii_alphanumeric() || *value == '-')
        .collect()
}
