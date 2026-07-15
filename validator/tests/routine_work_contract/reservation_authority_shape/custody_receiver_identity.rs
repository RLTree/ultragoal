use super::operations::BodyShape;

impl BodyShape {
    pub(super) fn require_identity_bound(&self) -> Result<(), &'static str> {
        let mut observed = self.self_paths.clone();
        observed.sort();
        let mut direct = self.direct_self_paths.clone();
        direct.sort();
        (observed == direct
            && self.constructor_locals == 0
            && self.hidden_custody == 0
            && self.import_renames == 0)
            .then_some(())
            .ok_or("authority-custody-receiver-alias")
    }

    pub(super) fn require_direct_transitions(&self) -> Result<(), &'static str> {
        self.indirect_transitions
            .is_empty()
            .then_some(())
            .ok_or("authority-custody-transition-alias")
    }
}
