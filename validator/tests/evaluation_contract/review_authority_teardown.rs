impl ReviewAuthorityHarness {
    fn teardown(self) -> Result<(), &'static str> {
        let Self {
            authority,
            ledger_root,
        } = self;
        drop(authority);
        teardown_private_ledger_root(&ledger_root)
    }
}

fn teardown_private_ledger_root(path: &Path) -> Result<(), &'static str> {
    let temp_root = std::env::temp_dir();
    if path.parent() != Some(temp_root.as_path()) {
        return Err("evaluation-contract-root-cleanup-outside-temp");
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("evaluation-contract-root-cleanup-name-invalid")?;
    let expected_prefix = format!("hul-evaluation-contract-review-{}-", std::process::id());
    if !name.starts_with(&expected_prefix) {
        return Err("evaluation-contract-root-cleanup-name-invalid");
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "evaluation-contract-root-cleanup-root-missing")?;
    if metadata.file_type().is_symlink()
        || !metadata.file_type().is_dir()
        || metadata.permissions().mode() & 0o777 != 0o700
    {
        return Err("evaluation-contract-root-cleanup-root-unsafe");
    }
    fs::remove_dir_all(path).map_err(|_| "evaluation-contract-root-cleanup-failed")
}
