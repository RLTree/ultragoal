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
    surface_content: Option<[String; 5]>,
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
    #[cfg(test)]
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
            surface_content: None,
        };
        bundle.validate()?;
        Ok(bundle)
    }

    pub(crate) fn from_observed_parts(
        installed_binding: String,
        cache_binding: String,
        registry_binding: String,
        discovery_binding: String,
        runtime_binding: String,
        installed_content: String,
        cache_content: String,
        registry_content: String,
        discovery_content: String,
        runtime_content: String,
        command_outcomes: Vec<HostCommandObservation>,
    ) -> Result<Self, ()> {
        let content = [
            installed_content,
            cache_content,
            registry_content,
            discovery_content,
            runtime_content,
        ];
        let bundle = Self {
            installed_sha256: observed_binding(&installed_binding, &content[0]),
            cache_sha256: observed_binding(&cache_binding, &content[1]),
            registry_sha256: observed_binding(&registry_binding, &content[2]),
            discovery_sha256: observed_binding(&discovery_binding, &content[3]),
            runtime_sha256: observed_binding(&runtime_binding, &content[4]),
            command_outcomes,
            surface_content: Some(content),
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
        let surfaces_match = match &self.surface_content {
            Some(content) => [
                (
                    &self.installed_sha256,
                    &expected.installed_sha256,
                    &content[0],
                ),
                (&self.cache_sha256, &expected.cache_sha256, &content[1]),
                (
                    &self.registry_sha256,
                    &expected.registry_sha256,
                    &content[2],
                ),
                (
                    &self.discovery_sha256,
                    &expected.discovery_sha256,
                    &content[3],
                ),
                (&self.runtime_sha256, &expected.runtime_sha256, &content[4]),
            ]
            .into_iter()
            .all(|(observed, binding, content)| observed == &observed_binding(binding, content)),
            None => {
                self.installed_sha256 == expected.installed_sha256
                    && self.cache_sha256 == expected.cache_sha256
                    && self.registry_sha256 == expected.registry_sha256
                    && self.discovery_sha256 == expected.discovery_sha256
                    && self.runtime_sha256 == expected.runtime_sha256
            }
        };
        surfaces_match && self.command_outcomes.len() == expected.command_count
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

fn observed_binding(binding: &str, content: &str) -> String {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(&(
        "harness-ultragoal.host-lifecycle-observation.v1",
        binding,
        content,
    ))
    .unwrap_or_default();
    format!("sha256:{:x}", Sha256::digest(bytes))
}
