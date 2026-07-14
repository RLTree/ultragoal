use super::*;

pub(crate) const DEFAULT_OUTPUT_LIMIT: usize = 1024 * 1024;
pub(crate) const DEFAULT_OBSERVED_OUTPUT_LIMIT: usize = 8 * 1024 * 1024;
pub(crate) const MAX_OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
pub(crate) const MAX_OBSERVED_OUTPUT_LIMIT: usize = 64 * 1024 * 1024;
pub(crate) const MAX_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);
pub(crate) const CATALOG_BINDING_UNAVAILABLE: &str = "capture unavailable: caller-supplied CommandSpec lacks an accepted typed-catalog binding and root-issued token";
pub(crate) const MAX_ARGUMENTS: usize = 4096;
pub(crate) const MAX_ARTIFACTS: usize = 128;
pub(crate) const MAX_ENVIRONMENT_ENTRIES: usize = 1024;
pub(crate) const MAX_PATH_BYTES: usize = 4096;

#[derive(Clone, Debug)]
pub(crate) struct CatalogBinding {
    pub command_id: String,
    pub capability: String,
}

pub(crate) struct CatalogPermit {
    pub(crate) binding: CatalogBinding,
}

impl CatalogPermit {
    #[cfg(not(test))]
    pub(crate) fn issue(
        descriptor: &'static crate::cli::successor::CommandDescriptor,
        capability: &'static str,
    ) -> Result<Self, String> {
        let canonical = crate::cli::successor::catalog()
            .iter()
            .find(|candidate| std::ptr::eq(*candidate, descriptor))
            .ok_or_else(|| "capture permit requires a canonical command descriptor".to_owned())?;
        if canonical.effect != EffectClass::Read || !supported_native_read(capability) {
            return Err("capture permit has no supported read-only native substrate".to_owned());
        }
        Ok(Self {
            binding: CatalogBinding {
                command_id: format!(
                    "{}-{}",
                    canonical.command.group().as_str(),
                    canonical.subcommand.unwrap_or("default")
                ),
                capability: capability.to_owned(),
            },
        })
    }
}

pub(crate) fn validate_path_byte_bound(path: &Path, label: &str) -> Result<(), String> {
    if os_bytes(path.as_os_str()).len() > MAX_PATH_BYTES {
        return Err(format!("{label} exceeds the supported path byte bound"));
    }
    Ok(())
}

/// A caller-supplied, structurally classified capture request. Paths remain
/// worktree-relative, and production capture stays fail-closed until root
/// supplies a typed-catalog binding to the native read-only host substrate.
pub struct CommandSpec {
    pub(crate) program: PathBuf,
    pub(crate) arguments: Vec<ArgumentInput>,
    pub(crate) cwd: PathBuf,
    pub(crate) effect: EffectClass,
    pub(crate) environment: Vec<EnvironmentInput>,
    pub(crate) artifacts: Vec<ArtifactExpectation>,
    pub(crate) timeout: Duration,
    pub(crate) output_limit: usize,
    pub(crate) observed_output_limit: usize,
    pub(crate) interrupt: Option<Arc<AtomicBool>>,
    pub(crate) catalog_binding: Option<CatalogBinding>,
}
