pub(crate) struct HostLifecycleBinding {
    package: PackageIdentity,
    command_plan: HostCommandPlan,
    scope_sha256: String,
    host_capability_sha256: String,
    expected_observations: HostLifecycleExpectedObservations,
}

impl HostLifecycleBinding {
    pub(crate) fn new(
        package: PackageIdentity,
        command_plan: HostCommandPlan,
        scope_sha256: String,
        host_capability_sha256: String,
        expected_observations: HostLifecycleExpectedObservations,
    ) -> Result<Self, LifecycleError> {
        package
            .validate()
            .map_err(|_| LifecycleError::InvalidTransition)?;
        if package != *command_plan.package()
            || !is_digest(&scope_sha256)
            || !is_digest(&host_capability_sha256)
            || expected_observations.validate().is_err()
        {
            return Err(LifecycleError::InvalidTransition);
        }
        Ok(Self {
            package,
            command_plan,
            scope_sha256,
            host_capability_sha256,
            expected_observations,
        })
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_plugin_ref(value: &str) -> bool {
    let Some((plugin, marketplace)) = value.split_once('@') else {
        return false;
    };
    plugin == "harness-ultragoal"
        && !marketplace.is_empty()
        && marketplace.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

fn absolute_path(value: &str) -> bool {
    let mut components = std::path::Path::new(value).components();
    matches!(components.next(), Some(std::path::Component::RootDir))
        && components.all(|component| matches!(component, std::path::Component::Normal(_)))
}
