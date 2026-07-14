/// Public research records are untrusted descriptions.  In particular, a
/// caller-controlled digest proves byte consistency, not who established the
/// source class, temporal policy, or observation time.  Keep that provenance
/// separate from the serialized record so recomputing canonical bytes cannot
/// mint review authority.
#[derive(Clone, Debug, Eq, PartialEq)]
enum ResearchSourceAuthorityBinding {
    UnverifiedExternal,
    #[cfg(test)]
    TestRootAdopted {
        record_sha256: String,
        source_class: ResearchSourceClass,
        observed_at_epoch_seconds: u64,
        valid_until_epoch_seconds: u64,
    },
}

impl ResearchSourceAuthorityBinding {
    fn validates(&self, _record: &ResearchSourceRecord, _record_bytes: &[u8]) -> bool {
        match self {
            Self::UnverifiedExternal => false,
            #[cfg(test)]
            Self::TestRootAdopted {
                record_sha256,
                source_class,
                observed_at_epoch_seconds,
                valid_until_epoch_seconds,
            } => {
                record_sha256 == &super::digest(_record_bytes)
                    && *source_class == _record.source_class
                    && *observed_at_epoch_seconds == _record.observed_at_epoch_seconds
                    && *valid_until_epoch_seconds == _record.valid_until_epoch_seconds
                    && source_class.supports_current_capability_decision()
                    && !_record.has_externally_untrusted_stable_fact()
            }
        }
    }
}
