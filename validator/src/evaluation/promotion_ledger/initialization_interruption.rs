#[cfg(test)]
fn promotion_initialization_interruption_hooks()
-> &'static std::sync::Mutex<std::collections::BTreeMap<PathBuf, &'static str>> {
    static HOOKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::BTreeMap<PathBuf, &'static str>>,
    > = std::sync::OnceLock::new();
    HOOKS.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeMap::new()))
}

#[cfg(test)]
impl FilePromotionReviewLedger {
    pub(crate) fn set_test_initialization_interruption(root: PathBuf, stage: &'static str) {
        assert!(matches!(
            stage,
            "lock" | "anchor_temporary" | "anchor_named" | "state_temporary" | "state_named"
        ));
        promotion_initialization_interruption_hooks()
            .lock()
            .expect("promotion initialization interruption hook lock")
            .insert(root, stage);
    }
}

#[cfg(test)]
fn test_promotion_initialization_interruption(
    root: &Path,
    stage: &'static str,
) -> Result<(), PromotionLedgerError> {
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
