impl std::fmt::Debug for PromotionReview {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PromotionReview")
            .field("contents", &"<redacted>")
            .finish()
    }
}

/// Implemented only by root-owned adapters inside this crate. The authority
/// supplies the current live binding and consumes each attestation once.
pub(crate) trait PromotionReviewAuthority {
    fn authority_id(&self) -> &str;
    fn reviewer_id(&self) -> &str;
    fn review_session_id(&self) -> &str;
    fn current_binding(&self) -> (&str, &str);
    fn issue_attestation(&mut self, binding_sha256: &str) -> Result<String, EvaluationError>;
    fn verify_and_consume(
        &mut self,
        binding_sha256: &str,
        reviewer_id: &str,
        review_id: &str,
        attestation_sha256: &str,
    ) -> bool;
}
