pub(super) struct IsolatedObservation {
    pub(super) installed_tree_sha256: String,
    pub(super) cache_observation_sha256: String,
    pub(super) marketplace_observation_sha256: String,
    pub(super) app_registry_observation_sha256: String,
    pub(super) runtime_observation_sha256: String,
    pub(super) journey_binding_sha256: String,
}

impl IsolatedObservation {
    pub(super) fn installed_tree_sha256(&self) -> &str {
        &self.installed_tree_sha256
    }

    pub(super) fn cache_observation_sha256(&self) -> &str {
        &self.cache_observation_sha256
    }

    pub(super) fn marketplace_observation_sha256(&self) -> &str {
        &self.marketplace_observation_sha256
    }

    pub(super) fn app_registry_observation_sha256(&self) -> &str {
        &self.app_registry_observation_sha256
    }

    pub(super) fn runtime_observation_sha256(&self) -> &str {
        &self.runtime_observation_sha256
    }

    pub(super) fn journey_binding_sha256(&self) -> &str {
        &self.journey_binding_sha256
    }
}
