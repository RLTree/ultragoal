#[cfg(test)]
use std::os::unix::fs::PermissionsExt;

#[cfg(test)]
fn promotion_initialization_interruption_hooks(
) -> &'static std::sync::Mutex<std::collections::BTreeMap<PathBuf, &'static str>> {
    static HOOKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::BTreeMap<PathBuf, &'static str>>,
    > = std::sync::OnceLock::new();
    HOOKS.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeMap::new()))
}

#[cfg(test)]
fn promotion_initialization_state_swap_roots(
) -> &'static std::sync::Mutex<std::collections::BTreeSet<PathBuf>> {
    static ROOTS: std::sync::OnceLock<std::sync::Mutex<std::collections::BTreeSet<PathBuf>>> =
        std::sync::OnceLock::new();
    ROOTS.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeSet::new()))
}

#[cfg(test)]
impl FilePromotionReviewLedger {
    pub(crate) fn set_test_initialization_interruption(root: PathBuf, stage: &'static str) {
        assert!(matches!(
            stage,
            "lock"
                | "anchor_temporary"
                | "anchor_named"
                | "state_reopen"
                | "state_temporary"
                | "state_named"
        ));
        promotion_initialization_interruption_hooks()
            .lock()
            .expect("promotion initialization interruption hook lock")
            .insert(root, stage);
    }

    pub(crate) fn set_test_state_scratch_swap(root: PathBuf) {
        promotion_initialization_state_swap_roots()
            .lock()
            .expect("promotion state scratch swap hook lock")
            .insert(root);
    }
}

#[cfg(test)]
fn test_promotion_initialization_interruption(
    root: &Path,
    stage: &'static str,
) -> Result<(), PromotionLedgerError> {
    #[cfg(test)]
    if stage == "state_reopen"
        && promotion_initialization_state_swap_roots()
            .lock()
            .expect("promotion state scratch swap hook lock")
            .remove(root)
    {
        let scratch = root.join(INITIAL_STATE_NAME);
        let replacement = root.join(".promotion-review.state.replacement");
        std::fs::rename(&scratch, &replacement).map_err(|_| {
            PromotionLedgerError::new("promotion-ledger-initialization-state-swap-failed")
        })?;
        std::fs::copy(&replacement, &scratch).map_err(|_| {
            PromotionLedgerError::new("promotion-ledger-initialization-state-swap-failed")
        })?;
        std::fs::set_permissions(&scratch, std::fs::Permissions::from_mode(0o600)).map_err(
            |_| PromotionLedgerError::new("promotion-ledger-initialization-state-swap-failed"),
        )?;
        std::fs::remove_file(replacement).map_err(|_| {
            PromotionLedgerError::new("promotion-ledger-initialization-state-swap-failed")
        })?;
    }
    let mut hooks = promotion_initialization_interruption_hooks()
        .lock()
        .expect("promotion initialization interruption hook lock");
    if hooks.get(root).copied() == Some(stage) {
        hooks.remove(root);
        return Err(PromotionLedgerError::new(
            "promotion-ledger-initialization-interrupted",
        ));
    }
    Ok(())
}

#[cfg(not(test))]
fn test_promotion_initialization_interruption(
    _: &Path,
    _: &'static str,
) -> Result<(), PromotionLedgerError> {
    Ok(())
}
