#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostLifecycleExpectedObservations {
    pub(crate) installed_sha256: String,
    pub(crate) cache_sha256: String,
    pub(crate) registry_sha256: String,
    pub(crate) discovery_sha256: String,
    pub(crate) runtime_sha256: String,
    pub(crate) command_count: usize,
}

impl HostLifecycleExpectedObservations {
    pub(crate) fn validate(&self) -> Result<(), ()> {
        let digests = [
            &self.installed_sha256,
            &self.cache_sha256,
            &self.registry_sha256,
            &self.discovery_sha256,
            &self.runtime_sha256,
        ];
        if digests.iter().any(|value| !is_digest(value)) {
            return Err(());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HostLifecycleObservedBundle {
    installed_sha256: String,
    cache_sha256: String,
    registry_sha256: String,
    discovery_sha256: String,
    runtime_sha256: String,
    command_outcomes: Vec<HostCommandObservation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HostCommandObservation {
    command_index: usize,
    outcome_sha256: String,
    exit_code: i32,
    attempts: u8,
}

impl HostCommandObservation {
    pub(crate) fn new(
        command_index: usize,
        outcome_sha256: String,
        exit_code: i32,
        attempts: u8,
    ) -> Result<Self, ()> {
        let observation = Self {
            command_index,
            outcome_sha256,
            exit_code,
            attempts,
        };
        if !is_digest(&observation.outcome_sha256) || observation.attempts != 1 {
            return Err(());
        }
        Ok(observation)
    }
}

impl HostLifecycleObservedBundle {
    pub(crate) fn from_parts(
        installed_sha256: String,
        cache_sha256: String,
        registry_sha256: String,
        discovery_sha256: String,
        runtime_sha256: String,
        command_outcomes: Vec<HostCommandObservation>,
    ) -> Result<Self, ()> {
        let bundle = Self {
            installed_sha256,
            cache_sha256,
            registry_sha256,
            discovery_sha256,
            runtime_sha256,
            command_outcomes,
        };
        bundle.validate()?;
        Ok(bundle)
    }

    fn validate(&self) -> Result<(), ()> {
        let digests = [
            &self.installed_sha256,
            &self.cache_sha256,
            &self.registry_sha256,
            &self.discovery_sha256,
            &self.runtime_sha256,
        ];
        if digests.iter().any(|value| !is_observation_digest(value))
            || self
                .command_outcomes
                .iter()
                .enumerate()
                .any(|(index, command)| {
                    command.command_index != index
                        || !is_digest(&command.outcome_sha256)
                        || command.attempts != 1
                })
        {
            return Err(());
        }
        Ok(())
    }

    pub(crate) fn matches(&self, expected: &HostLifecycleExpectedObservations) -> bool {
        self.installed_sha256 == expected.installed_sha256
            && self.cache_sha256 == expected.cache_sha256
            && self.registry_sha256 == expected.registry_sha256
            && self.discovery_sha256 == expected.discovery_sha256
            && self.runtime_sha256 == expected.runtime_sha256
            && self.command_outcomes.len() == expected.command_count
    }

    pub(crate) fn command_cursor(&self) -> usize {
        self.command_outcomes.len()
    }
}

fn is_observation_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
