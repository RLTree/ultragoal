//! Fixture-only process adapter.  This is intentionally separate from public
//! `CommandSpec`: fixture execution has a confined-write lease, while public
//! capture remains read-only and catalog-bound.

mod execute;
mod permit;

pub(crate) use permit::FixtureCaptureAdapter;

#[cfg(test)]
mod tests {
    use super::FixtureCaptureAdapter;
    use crate::fixture_scheduler::{
        ExpectedOutcome, FixtureKind, FixtureScheduler, FixtureSpec, ResourceKind, RunDisposition,
    };
    use std::collections::BTreeSet;
    use std::ffi::OsString;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "hul-fixture-capture-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn spec(id: &str, expected: ExpectedOutcome) -> FixtureSpec {
        FixtureSpec::new(
            id,
            if matches!(
                expected.verdict,
                crate::fixture_scheduler::OutcomeVerdict::Pass
            ) {
                FixtureKind::Positive
            } else {
                FixtureKind::Negative
            },
            "capture-execution",
            BTreeSet::from([ResourceKind::File, ResourceKind::Env, ResourceKind::Port]),
            expected,
            false,
        )
        .unwrap()
    }

    #[test]
    fn pinned_fixture_adapter_executes_and_derives_acceptance() {
        let root = root("accept");
        let fixture = spec("executed", ExpectedOutcome::pass(2));
        let adapter = FixtureCaptureAdapter::issue(
            &fixture,
            "/usr/bin/true".into(),
            Vec::<OsString>::new(),
            4096,
            Vec::new(),
        )
        .unwrap();
        let mut scheduler = FixtureScheduler::new(&root);
        let lease = scheduler.schedule([fixture]).unwrap().pop().unwrap();
        assert_eq!(
            scheduler.execute(&lease, &adapter).unwrap(),
            RunDisposition::Accepted
        );
        assert!(!root.join(lease).exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn nonzero_pinned_fixture_derives_only_the_bound_causal_control() {
        let root = root("negative");
        let fixture = spec(
            "negative",
            ExpectedOutcome::causal_failure("intentional", 1),
        );
        let adapter = FixtureCaptureAdapter::issue(
            &fixture,
            "/usr/bin/false".into(),
            Vec::<OsString>::new(),
            4096,
            Vec::new(),
        )
        .unwrap();
        let mut scheduler = FixtureScheduler::new(&root);
        let lease = scheduler.schedule([fixture]).unwrap().pop().unwrap();
        assert_eq!(
            scheduler.execute(&lease, &adapter).unwrap(),
            RunDisposition::CausalFailure
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
