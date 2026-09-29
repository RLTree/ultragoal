#[cfg(test)]
fn publication_hooks() -> &'static std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>>
{
    static HOOKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>>,
    > = std::sync::OnceLock::new();
    HOOKS.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeMap::new()))
}

#[cfg(test)]
fn test_publication_pause(root: &Path) {
    let milliseconds = {
        let mut hooks = publication_hooks()
            .lock()
            .expect("promotion publication hook lock");
        let Some(hook) = hooks.get_mut(root) else {
            return;
        };
        hook.paused = true;
        hook.milliseconds
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(milliseconds);
    loop {
        let mut hooks = publication_hooks()
            .lock()
            .expect("promotion publication hook lock");
        if hooks.get(root).is_none_or(|hook| hook.released) {
            hooks.remove(root);
            return;
        }
        drop(hooks);
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release promotion publication"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[cfg(not(test))]
fn test_publication_pause(_: &Path) {}

#[cfg(test)]
fn final_validation_hooks()
-> &'static std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>> {
    static HOOKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>>,
    > = std::sync::OnceLock::new();
    HOOKS.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeMap::new()))
}

#[cfg(test)]
fn test_final_validation_pause(root: &Path) {
    let milliseconds = {
        let mut hooks = final_validation_hooks()
            .lock()
            .expect("promotion final validation hook lock");
        let Some(hook) = hooks.get_mut(root) else {
            return;
        };
        hook.paused = true;
        hook.milliseconds
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(milliseconds);
    loop {
        let mut hooks = final_validation_hooks()
            .lock()
            .expect("promotion final validation hook lock");
        if hooks.get(root).is_none_or(|hook| hook.released) {
            hooks.remove(root);
            return;
        }
        drop(hooks);
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release promotion final validation"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[cfg(not(test))]
fn test_final_validation_pause(_: &Path) {}

#[cfg(test)]
fn directory_scan_hooks()
-> &'static std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>> {
    static HOOKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>>,
    > = std::sync::OnceLock::new();
    HOOKS.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeMap::new()))
}

#[cfg(test)]
fn test_directory_scan_pause(root: &Path) {
    let milliseconds = {
        let mut hooks = directory_scan_hooks()
            .lock()
            .expect("promotion directory scan hook lock");
        let Some(hook) = hooks.get_mut(root) else {
            return;
        };
        hook.paused = true;
        hook.milliseconds
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(milliseconds);
    loop {
        let mut hooks = directory_scan_hooks()
            .lock()
            .expect("promotion directory scan hook lock");
        if hooks.get(root).is_none_or(|hook| hook.released) {
            hooks.remove(root);
            return;
        }
        drop(hooks);
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release promotion directory scan"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[cfg(not(test))]
fn test_directory_scan_pause(_: &Path) {}

impl FilePromotionReviewLedger {
    pub(super) fn authority_id(&self) -> &str {
        &self.binding.authority_id
    }

    pub(super) fn reviewer_id(&self) -> &str {
        &self.binding.reviewer_id
    }

    pub(super) fn review_session_id(&self) -> &str {
        &self.binding.review_session_id
    }

    pub(super) fn current_review_binding(&self) -> (&str, &str) {
        (&self.binding.live_context_id, &self.binding.candidate_id)
    }
}

fn validate_binding(binding: &PromotionLedgerBinding) -> Result<(), PromotionLedgerError> {
    if binding.authority_id == binding.reviewer_id
        || binding.review_session_id == binding.baseline_execution_session_id
        || binding.review_session_id == binding.candidate_execution_session_id
        || binding.baseline_execution_session_id == binding.candidate_execution_session_id
        || !super::valid_identifier(&binding.authority_id)
        || !super::valid_identifier(&binding.reviewer_id)
        || [
            binding.review_session_id.as_str(),
            binding.live_context_id.as_str(),
            binding.baseline_candidate_id.as_str(),
            binding.candidate_id.as_str(),
            binding.baseline_run_sha256.as_str(),
            binding.candidate_run_sha256.as_str(),
            binding.baseline_execution_head_sha256.as_str(),
            binding.candidate_execution_head_sha256.as_str(),
        ]
        .iter()
        .any(|value| !super::valid_sha256(value))
    {
        return Err(PromotionLedgerError::new(
            "promotion-ledger-binding-invalid",
        ));
    }
    Ok(())
}
