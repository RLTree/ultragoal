pub(crate) fn canonical_temporary_parent() -> Result<std::path::PathBuf, DistributionError> {
    std::env::var_os("CODEX_WORKTREE_TMP")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .canonicalize()
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))
}
