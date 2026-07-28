pub fn execute_runtime_probe(
    plan: &RuntimeProbePlan,
) -> Result<RuntimeObservation, DistributionError> {
    if let Some(install) = &plan.install {
        install.revalidate(&plan.binding)?;
    }
    plan.executable.revalidate()?;
    refuse_unconfined_runtime_execution(&plan.executable.path)
}

fn refuse_unconfined_runtime_execution(
    _executable_path: &Path,
) -> Result<RuntimeObservation, DistributionError> {
    Err(error(DistributionErrorId::CapabilityMismatch))
}

#[cfg(test)]
mod runtime_execution_tests {
    use super::*;
    use std::fs;
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

        assert_eq!(
            refuse_unconfined_runtime_execution(&executable)
                .expect_err("ambient runtime execution must fail closed")
                .id(),
            DistributionErrorId::CapabilityMismatch
        );
        assert!(!marker.exists());
        fs::remove_dir_all(root).expect("test cleanup");
    }
}
