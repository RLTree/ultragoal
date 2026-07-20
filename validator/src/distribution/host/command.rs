const COMMAND_TIMEOUT_MS: u64 = 30_000;
const COMMAND_MAX_ATTEMPTS: u8 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HostCommand {
    program: String,
    argv: Vec<String>,
    environment: Vec<(String, String)>,
    timeout_ms: u64,
    max_attempts: u8,
}

impl HostCommand {
    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    pub fn environment(&self) -> &[(String, String)] {
        &self.environment
    }

    pub const fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    pub const fn max_attempts(&self) -> u8 {
        self.max_attempts
    }
}

pub struct HostCommandPlan {
    package: PackageIdentity,
    commands: Vec<HostCommand>,
    plan_sha256: String,
    consumed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Clone for HostCommandPlan {
    fn clone(&self) -> Self {
        Self {
            package: self.package.clone(),
            commands: self.commands.clone(),
            plan_sha256: self.plan_sha256.clone(),
            consumed: std::sync::Arc::clone(&self.consumed),
        }
    }
}

impl PartialEq for HostCommandPlan {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package
            && self.commands == other.commands
            && self.plan_sha256 == other.plan_sha256
    }
}

impl Eq for HostCommandPlan {}

impl HostCommandPlan {
    pub(super) fn consume_once(&self) -> bool {
        self.consumed
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_ok()
    }
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

fn command(argv: &[&str]) -> HostCommand {
    HostCommand {
        program: "codex".to_owned(),
        argv: argv.iter().map(|row| (*row).to_owned()).collect(),
        environment: Vec::new(),
        timeout_ms: COMMAND_TIMEOUT_MS,
        max_attempts: COMMAND_MAX_ATTEMPTS,
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
        consumed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
