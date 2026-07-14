#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PromotionDecision {
    pub baseline_candidate_id: String,
    pub candidate_id: String,
    pub reviewer_id: String,
    pub status: PromotionStatus,
    pub baseline_run_sha256: String,
    pub candidate_run_sha256: String,
    pub reasons: Vec<String>,
    pub claim_ceiling: String,
}
