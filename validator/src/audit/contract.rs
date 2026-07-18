pub use crate::contract_check_ids::CHECK_IDS;

pub const REQUIRED_SKILLS: &[&str] = &[
    "harness-ultragoal",
    "repository-fit",
    "routine-work",
    "diagnose-and-observe",
    "goal-run",
    "prove",
    "improve-and-maintain",
    "product-journey-review",
];

#[derive(Debug, Clone)]
pub struct Failure {
    pub check_id: String,
    pub error: String,
    pub detail: String,
}

impl Failure {
    pub fn new(check_id: &str, error: &str, detail: impl Into<String>) -> Self {
        Self {
            check_id: check_id.to_string(),
            error: error.to_string(),
            detail: detail.into(),
        }
    }
}
