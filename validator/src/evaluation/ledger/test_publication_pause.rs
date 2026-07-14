#[cfg(test)]
fn test_publication_pause(root: &Path) {
    let milliseconds = {
        let mut hooks = publication_hooks()
            .lock()
            .expect("evaluation publication hook lock");
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
            .expect("evaluation publication hook lock");
        if hooks.get(root).is_none_or(|hook| hook.released) {
            hooks.remove(root);
            return;
        }
        drop(hooks);
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release evaluation publication"
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
            .expect("evaluation final validation hook lock");
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
            .expect("evaluation final validation hook lock");
        if hooks.get(root).is_none_or(|hook| hook.released) {
            hooks.remove(root);
            return;
        }
        drop(hooks);
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release evaluation final validation"
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
            .expect("evaluation directory scan hook lock");
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
            .expect("evaluation directory scan hook lock");
        if hooks.get(root).is_none_or(|hook| hook.released) {
            hooks.remove(root);
            return;
        }
        drop(hooks);
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting to release evaluation directory scan"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[cfg(not(test))]
fn test_directory_scan_pause(_: &Path) {}

fn checked_causal_code(value: impl Into<String>) -> Result<String, EvaluationLedgerError> {
    let value = value.into();
    if !super::valid_identifier(&value) {
        return Err(EvaluationLedgerError::new(
            "evaluation-ledger-causal-code-invalid",
        ));
    }
    Ok(value)
}
