use super::artifact;
use super::inputs::{
    ArgumentInput, ArtifactExpectation, EnvironmentInput, PublicArg, PublicArtifact, PublicEnv,
    SecretArg, SecretArtifact, SecretEnv,
};
use super::run::{self, CapturedRun};
use super::util::{os_bytes, validate_public_path, validate_relative};
use crate::context::{EffectClass, LiveContext};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub(super) const DEFAULT_OUTPUT_LIMIT: usize = 1024 * 1024;
pub(super) const DEFAULT_OBSERVED_OUTPUT_LIMIT: usize = 8 * 1024 * 1024;
pub(super) const MAX_OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
pub(super) const MAX_OBSERVED_OUTPUT_LIMIT: usize = 64 * 1024 * 1024;
pub(super) const MAX_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);
pub(super) const CATALOG_BINDING_UNAVAILABLE: &str = "capture unavailable: caller-supplied CommandSpec lacks an accepted typed-catalog binding and root-issued token";
const MAX_ARGUMENTS: usize = 4096;
const MAX_ARTIFACTS: usize = 128;
const MAX_ENVIRONMENT_ENTRIES: usize = 1024;
pub(super) const MAX_PATH_BYTES: usize = 4096;

#[derive(Clone, Debug)]
pub(super) struct CatalogBinding {
    pub command_id: String,
    pub capability: String,
}

pub(crate) struct CatalogPermit {
    binding: CatalogBinding,
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

fn validate_path_byte_bound(path: &Path, label: &str) -> Result<(), String> {
    if os_bytes(path.as_os_str()).len() > MAX_PATH_BYTES {
        return Err(format!("{label} exceeds the supported path byte bound"));
    }
    Ok(())
}

/// A caller-supplied, structurally classified capture request. Paths remain
/// worktree-relative, and production capture stays fail-closed until root
/// supplies a typed-catalog binding to the native read-only host substrate.
pub struct CommandSpec {
    pub(super) program: PathBuf,
    pub(super) arguments: Vec<ArgumentInput>,
    pub(super) cwd: PathBuf,
    pub(super) effect: EffectClass,
    pub(super) environment: Vec<EnvironmentInput>,
    pub(super) artifacts: Vec<ArtifactExpectation>,
    pub(super) timeout: Duration,
    pub(super) output_limit: usize,
    pub(super) observed_output_limit: usize,
    pub(super) interrupt: Option<Arc<AtomicBool>>,
    pub(super) catalog_binding: Option<CatalogBinding>,
}

impl CommandSpec {
    pub fn new(program: impl Into<PathBuf>, effect: EffectClass) -> Self {
        Self {
            program: program.into(),
            arguments: Vec::new(),
            cwd: PathBuf::from("."),
            effect,
            environment: Vec::new(),
            artifacts: Vec::new(),
            timeout: Duration::from_secs(60),
            output_limit: DEFAULT_OUTPUT_LIMIT,
            observed_output_limit: DEFAULT_OBSERVED_OUTPUT_LIMIT,
            interrupt: None,
            catalog_binding: None,
        }
    }

    /// Issue a crate-internal, read-only capture request from the adopted
    /// successor command adapter. Public callers cannot mint this binding.
    #[cfg(test)]
    pub(crate) fn catalog_read(
        command_id: impl Into<String>,
        capability: impl Into<String>,
    ) -> Self {
        let capability = capability.into();
        assert!(supported_native_read(&capability));
        Self {
            program: PathBuf::from(&capability),
            arguments: Vec::new(),
            cwd: PathBuf::from("."),
            effect: EffectClass::Read,
            environment: Vec::new(),
            artifacts: Vec::new(),
            timeout: Duration::from_secs(60),
            output_limit: DEFAULT_OUTPUT_LIMIT,
            observed_output_limit: DEFAULT_OBSERVED_OUTPUT_LIMIT,
            interrupt: None,
            catalog_binding: Some(CatalogBinding {
                command_id: command_id.into(),
                capability,
            }),
        }
    }

    #[cfg(not(test))]
    pub(crate) fn catalog_read(permit: CatalogPermit) -> Self {
        let capability = permit.binding.capability.clone();
        Self {
            program: PathBuf::from(&capability),
            arguments: Vec::new(),
            cwd: PathBuf::from("."),
            effect: EffectClass::Read,
            environment: Vec::new(),
            artifacts: Vec::new(),
            timeout: Duration::from_secs(60),
            output_limit: DEFAULT_OUTPUT_LIMIT,
            observed_output_limit: DEFAULT_OBSERVED_OUTPUT_LIMIT,
            interrupt: None,
            catalog_binding: Some(permit.binding),
        }
    }

    pub fn public_arg(mut self, input: PublicArg) -> Self {
        self.arguments.push(ArgumentInput::Public(input));
        self
    }

    pub fn secret_arg(mut self, input: SecretArg) -> Self {
        self.arguments.push(ArgumentInput::Secret(input));
        self
    }

    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = path.into();
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn output_limit(mut self, bytes: usize) -> Self {
        self.output_limit = bytes;
        self
    }

    pub fn observed_output_limit(mut self, bytes: usize) -> Self {
        self.observed_output_limit = bytes;
        self
    }

    pub fn public_environment(mut self, input: PublicEnv) -> Self {
        self.environment.push(EnvironmentInput::Public(input));
        self
    }

    pub fn secret_environment(mut self, input: SecretEnv) -> Self {
        self.environment.push(EnvironmentInput::Secret(input));
        self
    }

    pub fn public_artifact(mut self, input: PublicArtifact) -> Self {
        self.artifacts.push(ArtifactExpectation::Public(input));
        self
    }

    pub fn secret_artifact(mut self, input: SecretArtifact) -> Self {
        self.artifacts.push(ArtifactExpectation::Secret(input));
        self
    }

    pub fn with_interrupt_flag(mut self, interrupted: Arc<AtomicBool>) -> Self {
        self.interrupt = Some(interrupted);
        self
    }

    pub fn run(&self, context: &LiveContext) -> Result<CapturedRun, String> {
        run::capture(self, context)
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        if self.arguments.len() > MAX_ARGUMENTS {
            return Err("argv exceeds the supported bound".to_owned());
        }
        if self.environment.len() > MAX_ENVIRONMENT_ENTRIES {
            return Err("environment allowlist exceeds entry bound".to_owned());
        }
        if self.artifacts.len() > MAX_ARTIFACTS {
            return Err("artifact expectation count exceeds the supported bound".to_owned());
        }
        validate_path_byte_bound(&self.program, "program")?;
        validate_path_byte_bound(&self.cwd, "cwd")?;
        validate_relative(&self.program, "program")?;
        validate_relative(&self.cwd, "cwd")?;
        validate_public_path(&self.program, "program")?;
        validate_public_path(&self.cwd, "cwd")?;
        artifact::validate_expectations(&self.artifacts)?;
        if self.timeout.is_zero() || self.timeout > MAX_TIMEOUT {
            return Err("timeout is outside the supported bound".to_owned());
        }
        if !(1..=MAX_OUTPUT_LIMIT).contains(&self.output_limit) {
            return Err("output limit is outside the supported bound".to_owned());
        }
        if !(self.output_limit..=MAX_OBSERVED_OUTPUT_LIMIT).contains(&self.observed_output_limit) {
            return Err("observed output limit is outside the supported bound".to_owned());
        }
        let argument_bytes = self.arguments.iter().fold(0_usize, |total, input| {
            let value = match input {
                ArgumentInput::Public(item) => &item.value,
                ArgumentInput::Secret(item) => &item.value,
            };
            total.saturating_add(os_bytes(value).len())
        });
        if argument_bytes > MAX_OUTPUT_LIMIT {
            return Err("argv exceeds the supported bound".to_owned());
        }
        if self.arguments.iter().any(|input| {
            let value = match input {
                ArgumentInput::Public(item) => &item.value,
                ArgumentInput::Secret(item) => &item.value,
            };
            os_bytes(value).contains(&0)
        }) {
            return Err("argv contains NUL".to_owned());
        }
        if self.effect != EffectClass::Read {
            return Err(
                "capture effect unavailable: catalog-unbound requests support only Read no-spawn observations"
                    .to_owned(),
            );
        }
        let has_caller_public_input = self
            .arguments
            .iter()
            .any(|input| matches!(input, ArgumentInput::Public(_)))
            || self
                .environment
                .iter()
                .any(|input| matches!(input, EnvironmentInput::Public(_)))
            || self
                .artifacts
                .iter()
                .any(|input| matches!(input, ArtifactExpectation::Public(_)));
        if self.catalog_binding.is_none() && has_caller_public_input {
            return Err(CATALOG_BINDING_UNAVAILABLE.to_owned());
        }
        if let Some(binding) = &self.catalog_binding
            && (!valid_catalog_id(&binding.command_id)
                || !valid_catalog_id(&binding.capability)
                || !supported_native_read(&binding.capability))
        {
            return Err("capture catalog binding is invalid".to_owned());
        }
        Ok(())
    }

    pub(super) fn require_catalog_binding(&self) -> Result<&CatalogBinding, String> {
        self.catalog_binding
            .as_ref()
            .ok_or_else(|| CATALOG_BINDING_UNAVAILABLE.to_owned())
    }
}

fn supported_native_read(capability: &str) -> bool {
    matches!(capability, "false" | "printf" | "sleep" | "true" | "yes")
}

fn valid_catalog_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}
