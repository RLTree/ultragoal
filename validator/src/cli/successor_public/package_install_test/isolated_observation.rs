pub(super) struct IsolatedObservation {
    pub(super) marketplace_source_tree_sha256: String,
    pub(super) installed_observation_sha256: String,
    pub(super) cache_observation_sha256: String,
    pub(super) marketplace_observation_sha256: String,
    pub(super) runtime_observation_sha256: String,
    pub(super) journey_binding_sha256: String,
}

impl IsolatedObservation {
    pub(super) fn marketplace_source_tree_sha256(&self) -> &str {
        &self.marketplace_source_tree_sha256
    }

    pub(super) fn cache_observation_sha256(&self) -> &str {
        &self.cache_observation_sha256
    }

    pub(super) fn installed_observation_sha256(&self) -> &str {
        &self.installed_observation_sha256
    }

    pub(super) fn marketplace_observation_sha256(&self) -> &str {
        &self.marketplace_observation_sha256
    }

    pub(super) fn runtime_observation_sha256(&self) -> &str {
        &self.runtime_observation_sha256
    }

    pub(super) fn journey_binding_sha256(&self) -> &str {
        &self.journey_binding_sha256
    }
}
