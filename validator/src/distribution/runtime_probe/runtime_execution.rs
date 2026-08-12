pub fn execute_runtime_probe(
    plan: &RuntimeProbePlan,
) -> Result<RuntimeObservation, DistributionError> {
    if let Some(install) = &plan.install {
        install.revalidate(&plan.binding)?;
    }
    plan.executable.revalidate()?;
    let (stdout, stderr) = crate::distribution::host_effect::execute_runtime_help(
        &plan.executable.path,
        plan.timeout,
        plan.executable.sha256(),
    )?;
    validate_envelope(&stdout)?;
    plan.executable.revalidate()?;
    if let Some(install) = &plan.install {
        install.revalidate(&plan.binding)?;
    }
    let mut output = stdout;
    output.extend_from_slice(&stderr);
    Ok(RuntimeObservation::executed(
        &plan.binding,
        plan.executable.sha256().to_owned(),
        sha256(&output),
    ))
}

#[cfg(test)]
mod runtime_execution_tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn package_program_is_not_executed_without_a_confined_effect_authority() {
        let root = std::env::temp_dir().join(format!(
            "hul-refused-runtime-probe-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("test root");
        let marker = root.join("marker");
        let executable = root.join("runtime");
        fs::write(
            &executable,
            format!("#!/bin/sh\nprintf invoked > '{}'\n", marker.display()),
        )
        .expect("test runtime");
        #[cfg(unix)]
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("runtime executable mode");

        assert!(!marker.exists());
        assert_eq!(
            crate::distribution::host_effect::execute_runtime_help(
                &executable,
                Duration::from_secs(10),
                "sha256:invalid",
            )
            .expect_err("unbound runtime must fail closed")
            .id(),
            DistributionErrorId::ProvenanceMismatch
        );
        assert!(!marker.exists());
        fs::remove_dir_all(root).expect("test cleanup");
    }
}
