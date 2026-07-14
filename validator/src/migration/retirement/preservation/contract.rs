impl RetirementDecision {
    pub(crate) fn reconcile_preservation<
        P: ReplacementEvidenceAuthority,
        R: RetirementReviewAuthority,
    >(
        plan: &MigrationPlan,
        target_id: &str,
        current: &MigrationInventory,
        review: &RetirementReview,
        replacement_authority: &mut P,
        review_authority: &mut R,
    ) -> Self {
        Self::reconcile_decision(
            plan,
            target_id,
            current,
            review,
            replacement_authority,
            review_authority,
            None,
        )
    }

    pub(crate) fn reconcile_destructive<
        P: ReplacementEvidenceAuthority,
        R: RetirementReviewAuthority,
        A: DestructiveEffectAuthority,
    >(
        plan: &MigrationPlan,
        target_id: &str,
        current: &MigrationInventory,
        review: &RetirementReview,
        authorization: &DestructiveAuthorization,
        replacement_authority: &mut P,
        review_authority: &mut R,
        effect_authority: &mut A,
    ) -> Self {
        Self::reconcile_decision(
            plan,
            target_id,
            current,
            review,
            replacement_authority,
            review_authority,
            Some((authorization, effect_authority)),
        )
    }
}
