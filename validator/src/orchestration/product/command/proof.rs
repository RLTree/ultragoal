use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOffer {
    UnitTest,
    FixtureReceipt,
    SerializedPermit,
    GeneratedRow,
    InProcessSimulation,
    RootIntegratedPublicCommand,
    FreshProcessObservation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceDisposition {
    SupportingOnly,
    RequiresIndependentRootReconciliation,
}

/// Prevents source, fixtures, permits, and simulations from being mistaken for
/// public command or representative journey proof.
pub const fn classify_evidence(offer: EvidenceOffer) -> EvidenceDisposition {
    match offer {
        EvidenceOffer::UnitTest
        | EvidenceOffer::FixtureReceipt
        | EvidenceOffer::SerializedPermit
        | EvidenceOffer::GeneratedRow
        | EvidenceOffer::InProcessSimulation => EvidenceDisposition::SupportingOnly,
        EvidenceOffer::RootIntegratedPublicCommand | EvidenceOffer::FreshProcessObservation => {
            EvidenceDisposition::RequiresIndependentRootReconciliation
        }
    }
}
