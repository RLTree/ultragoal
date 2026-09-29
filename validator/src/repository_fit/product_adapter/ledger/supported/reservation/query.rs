use super::*;

impl FileLedger {
    pub(crate) fn lookup_by_nonce(
        &self,
        nonce_sha256: &str,
    ) -> Result<Option<ExistingReservation>, LedgerError> {
        if !valid_digest(nonce_sha256) {
            return Err(invalid_transition());
        }
        self.with_snapshot(|_, replayed| {
            let existing = replayed
                .nonce_owner
                .get(nonce_sha256)
                .map(|owner| {
                    replayed
                        .records
                        .get(owner)
                        .map(existing)
                        .ok_or_else(tampered)
                })
                .transpose()?;
            Ok((existing, false))
        })
    }
}
