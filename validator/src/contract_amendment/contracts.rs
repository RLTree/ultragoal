pub(crate) struct CurrentAmendmentBinding<'a> {
    pub(crate) amendment_id: &'a str,
    pub(crate) amendment_hash: &'a str,
    pub(crate) contract_hash: &'a str,
    pub(crate) output_path: &'a str,
    pub(crate) output_hash: &'a str,
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
