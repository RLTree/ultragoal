impl std::fmt::Debug for PromotionReview {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PromotionReview")
            .field("contents", &"<redacted>")
            .finish()
    }
}
