use std::fmt;

/// Errors from advisory projections. These values never grant authority or
/// promote a claim; they only reject an incoherent proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdvisoryError {
    InvalidContract(&'static str),
    StaleBinding,
    PermissionWidening,
    MissingVerification,
    InvalidReview(&'static str),
    ClaimPromotion,
    AmbiguousRepair,
    UnchangedRepair,
    ProxyGaming,
    CandidateMismatch,
}

impl fmt::Display for AdvisoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidContract(reason) => reason,
            Self::StaleBinding => "advisory binding is stale",
            Self::PermissionWidening => "task evidence widens the leased or packaged scope",
            Self::MissingVerification => "task evidence omits required verification",
            Self::InvalidReview(reason) => reason,
            Self::ClaimPromotion => "review verdict cannot promote a claim",
            Self::AmbiguousRepair => "semantic repair evidence is ambiguous",
            Self::UnchangedRepair => {
                "semantic repair repeats the same mechanism without new evidence"
            }
            Self::ProxyGaming => "semantic repair uses a proxy signal instead of semantic evidence",
            Self::CandidateMismatch => "advisory candidate binding mismatch",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for AdvisoryError {}
