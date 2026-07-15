use super::super::*;

impl AttemptReservation {
    pub(crate) fn retain_non_durable_authentication(&self, artifacts: &BTreeMap<String, String>) {
        if self.durable.is_none() {
            registry()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .non_durable_authenticated_artifacts
                .extend(artifacts.clone());
        }
    }

    pub(crate) fn authenticates_artifact(
        &self,
        digest: &str,
        witness: &str,
    ) -> Result<bool, RoutineError> {
        if let Some(durable) = &self.durable {
            return durable.authenticates_artifact(digest, witness);
        }
        Ok(registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .non_durable_authenticated_artifacts
            .get(digest)
            .is_some_and(|expected| expected == witness))
    }
}
