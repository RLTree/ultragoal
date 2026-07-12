use crate::distribution::cache::MarketplaceEffects;
use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::model::PackageIdentity;
use crate::distribution::reader::sha256;
use crate::distribution::spec::digest;
use serde::Serialize;

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
        Err(()) => return Err(error(DistributionErrorId::RollbackFailed)),
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
    fn execute(&mut self, program: &str, argv: &[String]) -> Result<CommandOutput, ()>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HostExecutionSnapshot {
    context_id: String,
    candidate_id: String,
    plan_sha256: String,
    command_count: usize,
    output_sha256: Vec<String>,
}

pub fn execute_authorized(
    plan: &HostCommandPlan,
    authorization: &HostAuthorization,
    executor: &mut impl HostExecutor,
) -> Result<HostExecutionSnapshot, DistributionError> {
    if authorization.plan_sha256 != plan.plan_sha256
        || authorization.context_id != plan.package.source().context_id()
        || authorization.candidate_id != plan.package.source().candidate_id()
    {
        return Err(error(DistributionErrorId::EffectFailed));
    }
    let mut output_sha256 = Vec::with_capacity(plan.commands.len());
    for row in &plan.commands {
        let output = executor
            .execute(&row.program, &row.argv)
            .map_err(|_| error(DistributionErrorId::EffectFailed))?;
        if output.stdout.len() > OUTPUT_LIMIT
            || output.stderr.len() > OUTPUT_LIMIT
            || output.exit_code != 0
        {
            return Err(error(DistributionErrorId::EffectFailed));
        }
        let mut combined = output.stdout;
        combined.extend_from_slice(&output.stderr);
        output_sha256.push(sha256(&combined));
    }
    Ok(HostExecutionSnapshot {
        context_id: authorization.context_id.clone(),
        candidate_id: authorization.candidate_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        command_count: plan.commands.len(),
        output_sha256,
    })
}

fn command(argv: &[&str]) -> HostCommand {
    HostCommand {
        program: "codex".to_owned(),
        argv: argv.iter().map(|row| (*row).to_owned()).collect(),
    }
}

fn bound_plan(
    package: &PackageIdentity,
    commands: Vec<HostCommand>,
) -> Result<HostCommandPlan, DistributionError> {
    package.validate()?;
    #[derive(Serialize)]
    struct Binding<'a> {
        schema: &'static str,
        package: &'a PackageIdentity,
        commands: &'a [HostCommand],
    }
    let binding = Binding {
        schema: "harness-ultragoal.host-command-plan.v1",
        package,
        commands: &commands,
    };
    let plan_sha256 = serde_json::to_vec(&binding)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| error(DistributionErrorId::InvalidSpec))?;
    Ok(HostCommandPlan {
        package: package.clone(),
        commands,
        plan_sha256,
    })
}

fn validate_name(value: &str) -> Result<(), DistributionError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(())
}

fn validate_path_argument(value: &str) -> Result<(), DistributionError> {
    use std::path::Component;
    let mut components = std::path::Path::new(value).components();
    let shape = matches!(components.next(), Some(Component::RootDir))
        && components.all(|row| matches!(row, Component::Normal(_)));
    if value.is_empty()
        || value.len() > 4096
        || value
            .bytes()
            .any(|byte| byte == 0 || byte.is_ascii_control())
        || !shape
    {
        return Err(error(DistributionErrorId::InvalidPath));
    }
    Ok(())
}
