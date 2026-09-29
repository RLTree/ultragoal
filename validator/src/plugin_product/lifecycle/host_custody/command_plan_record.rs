#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostCommandPlanRecord {
    package: PackageIdentity,
    commands: Vec<HostCommand>,
    plan_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHostCommandPlanRecord {
    package: PackageIdentity,
    commands: Vec<RawHostCommand>,
    plan_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHostCommand {
    program: String,
    argv: Vec<String>,
    environment: Vec<(String, String)>,
    timeout_ms: u64,
    max_attempts: u8,
}

impl<'de> Deserialize<'de> for HostCommandPlanRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawHostCommandPlanRecord::deserialize(deserializer)?;
        let commands = raw
            .commands
            .into_iter()
            .map(|command| {
                HostCommand::from_untrusted_record(
                    command.program,
                    command.argv,
                    command.environment,
                    command.timeout_ms,
                    command.max_attempts,
                )
            })
            .collect();
        Ok(Self {
            package: raw.package,
            commands,
            plan_sha256: raw.plan_sha256,
        })
    }
}

impl HostCommandPlanRecord {
    fn from_plan(plan: &HostCommandPlan) -> Self {
        Self {
            package: plan.package().clone(),
            commands: plan.commands().to_vec(),
            plan_sha256: plan.plan_sha256().to_owned(),
        }
    }

    fn validate(&self) -> Result<(), LifecycleError> {
        self.package
            .validate()
            .map_err(|_| LifecycleError::InvalidTransition)?;
        if self.commands.is_empty() || self.commands.len() > 2 {
            return Err(LifecycleError::InvalidTransition);
        }
        for command in &self.commands {
            if command.program() != "codex"
                || command.timeout_ms() != 30_000
                || command.max_attempts() != 1
                || !valid_isolated_environment(command.environment())
            {
                return Err(LifecycleError::InvalidTransition);
            }
        }
        if self.commands.len() == 1 {
            let argv = self.commands[0].argv();
            if argv.len() != 3
                || argv[0] != "plugin"
                || !matches!(argv[1].as_str(), "add" | "remove")
                || !valid_plugin_ref(&argv[2])
            {
                return Err(LifecycleError::InvalidTransition);
            }
        } else {
            let marketplace = self.commands[1].argv();
            let add = self.commands[0].argv();
            if add.len() != 4
                || add[..3] != ["plugin", "marketplace", "add"]
                || !absolute_path(&add[3])
                || marketplace.len() != 3
                || marketplace[..2] != ["plugin", "add"]
                || !valid_plugin_ref(&marketplace[2])
            {
                return Err(LifecycleError::InvalidTransition);
            }
        }
        #[derive(Serialize)]
        struct Binding<'a> {
            schema: &'static str,
            package: &'a PackageIdentity,
            commands: &'a [HostCommand],
        }
        let digest = serde_json::to_vec(&Binding {
            schema: "harness-ultragoal.host-command-plan.v1",
            package: &self.package,
            commands: &self.commands,
        })
        .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
        .map_err(|_| LifecycleError::InvalidTransition)?;
        if digest != self.plan_sha256 {
            return Err(LifecycleError::InvalidTransition);
        }
        Ok(())
    }

    pub(crate) fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }
}

fn valid_isolated_environment(environment: &[(String, String)]) -> bool {
    if environment.is_empty() {
        return true;
    }
    if environment.len() != 2
        || environment[0].0 != "CODEX_HOME"
        || environment[1].0 != "HOME"
        || environment[0].1 != environment[1].1
    {
        return false;
    }
    let path = std::path::Path::new(&environment[0].1);
    path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
}
