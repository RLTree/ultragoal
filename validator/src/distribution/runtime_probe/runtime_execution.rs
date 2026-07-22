pub fn execute_runtime_probe(
    plan: &RuntimeProbePlan,
) -> Result<RuntimeObservation, DistributionError> {
    if let Some(install) = &plan.install {
        install.revalidate(&plan.binding)?;
    }
    plan.executable.revalidate()?;
    let execution_copy = plan.executable.execution_copy()?;
    let result = execute_runtime_probe_copy(plan, execution_copy.path());
    let cleanup = execution_copy.remove();
    cleanup?;
    result
}

fn execute_runtime_probe_copy(
    plan: &RuntimeProbePlan,
    executable_path: &Path,
) -> Result<RuntimeObservation, DistributionError> {
    let mut command = if plan.executable.shell_script() {
        let mut command = Command::new("/bin/sh");
        command.arg(executable_path);
        command
    } else {
        Command::new(executable_path)
    };
    command
        .args(HELP_ARGUMENTS)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|_| error(DistributionErrorId::EffectFailed))?;
    let deadline = Instant::now() + plan.timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error(DistributionErrorId::EffectFailed));
            }
            Err(_) => return Err(error(DistributionErrorId::EffectFailed)),
        }
    };
    let stdout = read_output(child.stdout.take())?;
    let stderr = read_output(child.stderr.take())?;
    if !status.success() {
        return Err(error(DistributionErrorId::EffectFailed));
    }
    let envelope: ProbeEnvelope = json::parse(&stdout, 64 * 1024)?;
    validate_envelope(&envelope)?;
    plan.executable.revalidate()?;
    if let Some(install) = &plan.install {
        install.revalidate(&plan.binding)?;
    }
    let mut output = stdout;
    output.extend_from_slice(&stderr);
    Ok(RuntimeObservation::executed(
        &plan.binding,
        plan.executable_sha256.clone(),
        sha256(&output),
    ))
}
