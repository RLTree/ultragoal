use super::plugin_product::product_fitness::{
    ClaimCeiling, DimensionDisposition, DimensionEvidence, EvidenceBinding, EvidenceClass,
    FitnessDimension, ManualJourneyRow, OperatorKind, OverallDisposition,
    ProductFitnessDisposition, ProductFitnessError, PublicEntryObservation, RealWorkObservation,
    SurfaceIdentities, SurfaceIdentity, SurfaceStatus, TRUTH_LAYERS, TruthLayer,
    source_candidate_ceilings,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

include!("candidate.rs");
include!("v2_real_use.rs");

include!("reviewer_is_disjoint_falsification_only_and_cannot_raise_claims.rs");
