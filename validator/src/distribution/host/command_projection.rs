#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HostCommandPlanProjection {
    plan_sha256: String,
    argv_sha256: String,
    environment: Vec<(String, String)>,
    command_count: usize,
}

impl HostCommandPlan {
    pub(crate) fn projection(&self) -> Result<HostCommandPlanProjection, DistributionError> {
        if self.commands.is_empty()
            || self
                .commands
                .iter()
                .any(|command| command.program() != "codex" || command.argv().is_empty())
        {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        #[derive(Serialize)]
        struct ExactArgv<'a> {
            schema: &'static str,
            commands: &'a [HostCommand],
            shell: bool,
            inherited_environment: bool,
            output_limit_bytes: u64,
            timeout_required: bool,
        }
        let argv_sha256 = serde_json::to_vec(&ExactArgv {
            schema: "harness-ultragoal.exact-host-command-argv.v1",
            commands: &self.commands,
            shell: false,
            inherited_environment: false,
            output_limit_bytes: 1024 * 1024,
            timeout_required: true,
        })
        .map(|bytes| sha256(&bytes))
        .map_err(|_| error(DistributionErrorId::InvalidSpec))?;
        let environment = self
            .commands
            .first()
            .map(|command| command.environment().to_vec())
            .ok_or_else(|| error(DistributionErrorId::InvalidSpec))?;
        if self
            .commands
            .iter()
            .any(|command| command.environment() != environment.as_slice())
        {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        Ok(HostCommandPlanProjection {
            plan_sha256: self.plan_sha256.clone(),
            argv_sha256,
            environment,
            command_count: self.commands.len(),
        })
    }
}

impl HostCommandPlanProjection {
    pub(crate) fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub(crate) fn argv_sha256(&self) -> &str {
        &self.argv_sha256
    }

    pub(crate) fn environment(&self) -> &[(String, String)] {
        &self.environment
    }

    pub(crate) const fn command_count(&self) -> usize {
        self.command_count
    }
}
