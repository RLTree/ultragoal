use super::*;

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

    pub(crate) fn validate(&self) -> Result<(), String> {
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

    pub(crate) fn require_catalog_binding(&self) -> Result<&CatalogBinding, String> {
        self.catalog_binding
            .as_ref()
            .ok_or_else(|| CATALOG_BINDING_UNAVAILABLE.to_owned())
    }
}

pub(crate) fn supported_native_read(capability: &str) -> bool {
    matches!(capability, "false" | "printf" | "sleep" | "true" | "yes")
}

pub(crate) fn valid_catalog_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}
