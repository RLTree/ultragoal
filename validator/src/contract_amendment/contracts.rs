pub(crate) struct CurrentAmendmentBinding<'a> {
    pub(crate) amendment_id: &'a str,
    pub(crate) amendment_hash: &'a str,
    pub(crate) previous_contract_hash: &'a str,
    pub(crate) new_contract_hash: &'a str,
    pub(crate) change_class: &'a str,
    pub(crate) backlog_updates: &'a [ExpectedArtifactBinding<'a>],
}

pub(crate) struct ExpectedArtifactBinding<'a> {
    pub(crate) path: &'a str,
    pub(crate) digest: &'a str,
}

pub(crate) struct ValidatedCurrentAmendment {
    amendment_id: String,
    amendment_hash: String,
}

impl ValidatedCurrentAmendment {
    pub(crate) fn amendment_id(&self) -> &str {
        &self.amendment_id
    }

    pub(crate) fn amendment_hash(&self) -> &str {
        &self.amendment_hash
    }

    pub(super) fn new(amendment_id: String, amendment_hash: String) -> Self {
        Self {
            amendment_id,
            amendment_hash,
        }
    }
}
