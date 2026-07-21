impl HostEffectLedgerError {
    pub(in crate::distribution::host_effect) const fn new(id: HostEffectLedgerErrorId) -> Self {
        Self { id }
    }

    #[cfg(test)]
    pub(crate) const fn id(&self) -> HostEffectLedgerErrorId {
        self.id
    }
}
