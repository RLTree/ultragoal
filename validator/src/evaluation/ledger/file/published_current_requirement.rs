impl FileEvaluationExecutionLedger {
    fn require_published_current(
        &self,
        expected: &AuthenticatedSnapshot,
    ) -> Result<(), EvaluationLedgerError> {
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        if pending.is_some()
            || current.partial_tail_from.is_some()
            || current.snapshot.head_sha256 != expected.head_sha256
            || current.observed_state_head_sha256 != expected.head_sha256
            || current.observed_anchor != expected.payload.anchor_observation
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-final-current-changed",
            ));
        }
        self.validate_descriptors()
    }

    fn validate_descriptors(&self) -> Result<(), EvaluationLedgerError> {
        let root = safe_file_identity(&self.root)?;
        let lock = safe_file_identity(&self.lock)?;
        let anchor = safe_file_identity(&self.anchor)?;
        let named_root = open_safe_directory(&self.root_path)
            .map_err(|_| EvaluationLedgerError::new("evaluation-ledger-descriptor-substituted"))?
            .1;
        if root.device != self.root_identity.device
            || root.inode != self.root_identity.inode
            || root.mode != self.root_identity.mode
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-root-descriptor-substituted",
            ));
        }
        if lock != self.lock_identity {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-lock-descriptor-substituted",
            ));
        }
        if anchor.authority() != self.anchor_authority {
            return Err(EvaluationLedgerError::new(
                "evaluation-anchor-descriptor-substituted",
            ));
        }
        require_named_lock_identity(&self.root, self.lock_identity)?;
        require_named_anchor_authority(&self.root, self.anchor_authority)?;
        if named_root.device != root.device
            || named_root.inode != root.inode
            || named_root.mode != root.mode
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-descriptor-substituted",
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn set_test_publication_pause(root: PathBuf, milliseconds: u64) {
        publication_hooks()
            .lock()
            .expect("evaluation publication hook lock")
            .insert(root, TestPause::new(milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_publication_is_paused(root: &Path) -> bool {
        publication_hooks()
            .lock()
            .expect("evaluation publication hook lock")
            .get(root)
            .is_some_and(|hook| hook.paused)
    }

    #[cfg(test)]
    pub(crate) fn release_test_publication(root: &Path) {
        if let Some(hook) = publication_hooks()
            .lock()
            .expect("evaluation publication hook lock")
            .get_mut(root)
        {
            hook.released = true;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_test_final_validation_pause(root: PathBuf, milliseconds: u64) {
        final_validation_hooks()
            .lock()
            .expect("evaluation final validation hook lock")
            .insert(root, TestPause::new(milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_final_validation_is_paused(root: &Path) -> bool {
        final_validation_hooks()
            .lock()
            .expect("evaluation final validation hook lock")
            .get(root)
            .is_some_and(|hook| hook.paused)
    }

    #[cfg(test)]
    pub(crate) fn release_test_final_validation(root: &Path) {
        if let Some(hook) = final_validation_hooks()
            .lock()
            .expect("evaluation final validation hook lock")
            .get_mut(root)
        {
            hook.released = true;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_test_directory_scan_pause(root: PathBuf, milliseconds: u64) {
        directory_scan_hooks()
            .lock()
            .expect("evaluation directory scan hook lock")
            .insert(root, TestPause::new(milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_directory_scan_is_paused(root: &Path) -> bool {
        directory_scan_hooks()
            .lock()
            .expect("evaluation directory scan hook lock")
            .get(root)
            .is_some_and(|hook| hook.paused)
    }

    #[cfg(test)]
    pub(crate) fn release_test_directory_scan(root: &Path) {
        if let Some(hook) = directory_scan_hooks()
            .lock()
            .expect("evaluation directory scan hook lock")
            .get_mut(root)
        {
            hook.released = true;
        }
    }
}

#[cfg(test)]
struct TestPause {
    milliseconds: u64,
    paused: bool,
    released: bool,
}

#[cfg(test)]
impl TestPause {
    fn new(milliseconds: u64) -> Self {
        Self {
            milliseconds,
            paused: false,
            released: false,
        }
    }
}

#[cfg(test)]
fn publication_hooks() -> &'static std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>>
{
    static HOOKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::BTreeMap<PathBuf, TestPause>>,
    > = std::sync::OnceLock::new();
    HOOKS.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeMap::new()))
}
