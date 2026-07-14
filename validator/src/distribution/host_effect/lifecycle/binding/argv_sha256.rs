fn argv_sha256(plan: &HostCommandPlan) -> Result<String, SupportedHostLifecycleError> {
    #[derive(Serialize)]
    struct ExactArgv<'a> {
        schema: &'static str,
        commands: &'a [crate::distribution::HostCommand],
        shell: bool,
        inherited_environment: bool,
        output_limit_bytes: u64,
        timeout_required: bool,
    }
    if plan.commands().is_empty()
        || plan
            .commands()
            .iter()
            .any(|command| command.program() != "codex" || command.argv().is_empty())
    {
        return Err(invalid());
    }
    digest_json(&ExactArgv {
        schema: "harness-ultragoal.exact-host-command-argv.v1",
        commands: plan.commands(),
        shell: false,
        inherited_environment: false,
        output_limit_bytes: 1024 * 1024,
        timeout_required: true,
    })
}

fn validate_name(value: &str) -> Result<(), SupportedHostLifecycleError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(invalid());
    }
    Ok(())
}

fn project_identity(value: &str) -> Result<String, SupportedHostLifecycleError> {
    let canonical = Path::new(value).canonicalize().map_err(|_| invalid())?;
    if Path::new(value) != canonical {
        // The argv path itself is effectful input. Reject aliases so a stable
        // target descriptor cannot be paired with a later-resolved symlink.
        return Err(invalid());
    }
    let metadata = std::fs::symlink_metadata(&canonical).map_err(|_| invalid())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(invalid());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(digest_bytes(
            format!(
                "{}\0{}\0{}",
                canonical.display(),
                metadata.dev(),
                metadata.ino()
            )
            .as_bytes(),
        ))
    }
    #[cfg(not(unix))]
    {
        Ok(digest_bytes(
            format!("{}\0{}", canonical.display(), metadata.len()).as_bytes(),
        ))
    }
}

fn digest_json(value: &impl Serialize) -> Result<String, SupportedHostLifecycleError> {
    serde_json::to_vec(value)
        .map(|bytes| digest_bytes(&bytes))
        .map_err(|_| invalid())
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn invalid() -> SupportedHostLifecycleError {
    lifecycle_error(SupportedHostLifecycleErrorId::InvalidAcceptedIdentity)
}
