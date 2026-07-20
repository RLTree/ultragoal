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
                || !command.environment().is_empty()
                || command.timeout_ms() != 30_000
                || command.max_attempts() != 1
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostLifecycleRecord {
    schema_version: String,
    issuance_id: u64,
    plan_id: String,
    intent: LifecycleIntent,
    before: LifecycleState,
    expected_after: LifecycleState,
    authorization_sha256: String,
    rollback_state: LifecycleState,
    effects: Vec<LifecycleEffect>,
    writes_host_state: bool,
    package: PackageIdentity,
    command_plan: HostCommandPlanRecord,
    scope_sha256: String,
    host_capability_sha256: String,
    command_cursor: usize,
    effect_cursor: usize,
    custody_phase: u8,
    expected_observations: HostLifecycleExpectedObservations,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct HostEffectExecutionBinding {
    record: HostLifecycleRecord,
}

impl HostEffectExecutionBinding {
    pub(crate) fn record(&self) -> &HostLifecycleRecord {
        &self.record
    }
}

impl HostLifecycleRecord {
    pub(crate) fn validate(&self) -> Result<(), LifecycleError> {
        if self.schema_version != "HarnessPluginHostLifecycleRecord-v2"
            || self.issuance_id == 0
            || self.effects.is_empty()
            || super::plan::record_writes_host_state(&self.effects) != self.writes_host_state
            || self.command_cursor > self.command_plan.commands.len()
            || self.effect_cursor > self.effects.len()
            || self.custody_phase > 2
            || !is_digest(&self.scope_sha256)
            || !is_digest(&self.host_capability_sha256)
            || self.expected_observations.validate().is_err()
            || self.command_plan.validate().is_err()
            || self.command_plan.package != self.package
        {
            return Err(LifecycleError::InvalidTransition);
        }
        self.package
            .validate()
            .map_err(|_| LifecycleError::InvalidTransition)?;
        self.before.validate()?;
        self.expected_after.validate()?;
        self.rollback_state.validate()?;
        super::model::validate_digest(&self.authorization_sha256)?;
        if self.rollback_state != self.before
            || super::plan::plan_digest(
                self.intent,
                &self.before,
                &self.expected_after,
                &self.effects,
                self.writes_host_state,
                &self.authorization_sha256,
            )? != self.plan_id
        {
            return Err(LifecycleError::InvalidTransition);
        }
        Ok(())
    }

    pub(crate) fn permit_join(&self) -> (&str, LifecycleIntent) {
        (&self.plan_id, self.intent)
    }

    pub(crate) fn expected_observations(&self) -> &HostLifecycleExpectedObservations {
        &self.expected_observations
    }

    pub(crate) fn package(&self) -> &PackageIdentity {
        &self.package
    }

    pub(crate) fn command_plan_sha256(&self) -> &str {
        self.command_plan.plan_sha256()
    }

    pub(crate) const fn command_cursor(&self) -> usize {
        self.command_cursor
    }

    pub(crate) const fn custody_phase(&self) -> u8 {
        self.custody_phase
    }

    pub(crate) const fn effect_cursor(&self) -> usize {
        self.effect_cursor
    }
}

use sha2::{Digest, Sha256};
