use super::*;

impl ProductionRoutineIssuer {
    pub(crate) fn seed_authority_capacity(
        &self,
        protocol_effect_count: usize,
        consumed_grant_count: usize,
    ) -> Result<(), RoutineError> {
        self.ledger
            .test_seed_capacity(protocol_effect_count, consumed_grant_count)
    }
}
