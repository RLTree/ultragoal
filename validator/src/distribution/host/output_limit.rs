const OUTPUT_LIMIT: usize = 1024 * 1024;
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HostCommand {
    program: String,
    argv: Vec<String>,
}

impl HostCommand {
    pub fn program(&self) -> &str {
        &self.program
    }
    pub fn argv(&self) -> &[String] {
        &self.argv
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct HostCommandPlan {
    package: PackageIdentity,
    commands: Vec<HostCommand>,
    plan_sha256: String,
}

impl std::fmt::Debug for HostCommandPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostCommandPlan")
            .field("package", &self.package)
            .field("command_count", &self.commands.len())
            .field("plan_sha256", &self.plan_sha256)
            .finish()
    }
}

pub struct MarketplaceTransaction {
    candidate_sha256: String,
    previous: Option<Vec<u8>>,
}

impl MarketplaceTransaction {
    pub(crate) fn new(candidate_sha256: String, previous: Option<Vec<u8>>) -> Self {
        Self {
            candidate_sha256,
            previous,
        }
    }
}

impl std::fmt::Debug for MarketplaceTransaction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MarketplaceTransaction")
            .field("candidate_sha256", &self.candidate_sha256)
            .field("had_previous", &self.previous.is_some())
            .finish()
    }
}

pub fn rollback_marketplace(
    transaction: MarketplaceTransaction,
    effects: &mut impl MarketplaceEffects,
) -> Result<(), DistributionError> {
    match effects.compare_exchange(
        Some(&transaction.candidate_sha256),
        transaction.previous.as_deref(),
    ) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(_) => return Err(error(DistributionErrorId::RollbackFailed)),
    }
    let observed = effects
        .read(1024 * 1024)
        .map_err(|_| error(DistributionErrorId::RollbackFailed))?;
    if observed != transaction.previous {
        return Err(error(DistributionErrorId::RollbackFailed));
    }
    Ok(())
}

impl HostCommandPlan {
    pub fn repository_install(
        package: &PackageIdentity,
        repository_root: &str,
        repository_marketplace: &str,
    ) -> Result<Self, DistributionError> {
        validate_path_argument(repository_root)?;
        validate_name(repository_marketplace)?;
        let commands = vec![
            command(&["plugin", "marketplace", "add", repository_root]),
            command(&[
                "plugin",
                "add",
                &format!("harness-ultragoal@{repository_marketplace}"),
            ]),
        ];
        bound_plan(package, commands)
    }

    pub fn personal_install(
        package: &PackageIdentity,
        marketplace: &str,
    ) -> Result<Self, DistributionError> {
        host_plugin_plan(package, "add", marketplace)
    }

    pub fn personal_remove(
        package: &PackageIdentity,
        marketplace: &str,
    ) -> Result<Self, DistributionError> {
        host_plugin_plan(package, "remove", marketplace)
    }

    pub fn repository_remove(
        package: &PackageIdentity,
        marketplace: &str,
    ) -> Result<Self, DistributionError> {
        validate_name(marketplace)?;
        let commands = vec![
            command(&[
                "plugin",
                "remove",
                &format!("harness-ultragoal@{marketplace}"),
            ]),
            command(&["plugin", "marketplace", "remove", marketplace]),
        ];
        bound_plan(package, commands)
    }

    pub fn commands(&self) -> &[HostCommand] {
        &self.commands
    }
    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }
}

fn host_plugin_plan(
    package: &PackageIdentity,
    action: &str,
    marketplace: &str,
) -> Result<HostCommandPlan, DistributionError> {
    validate_name(marketplace)?;
    let commands = vec![command(&[
        "plugin",
        action,
        &format!("harness-ultragoal@{marketplace}"),
    ])];
    bound_plan(package, commands)
}

#[derive(Clone, Debug)]
pub struct HostAuthorization {
    context_id: String,
    candidate_id: String,
    plan_sha256: String,
}

impl HostAuthorization {
    pub fn new(
        context_id: String,
        candidate_id: String,
        plan_sha256: String,
    ) -> Result<Self, DistributionError> {
        if !digest(&context_id) || !digest(&candidate_id) || !digest(&plan_sha256) {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        Ok(Self {
            context_id,
            candidate_id,
            plan_sha256,
        })
    }
}

pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub trait HostExecutor {
    fn execute(
        &mut self,
        program: &str,
        argv: &[String],
    ) -> Result<CommandOutput, crate::distribution::EffectFailure>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HostExecutionSnapshot {
    context_id: String,
    candidate_id: String,
    plan_sha256: String,
    command_count: usize,
    output_sha256: Vec<String>,
}
