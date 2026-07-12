use super::model::{ClaimCeiling, ProductFitnessError, TRUTH_LAYERS, TruthLayer};
use std::collections::BTreeMap;

pub fn source_candidate_ceilings() -> BTreeMap<TruthLayer, ClaimCeiling> {
    TRUTH_LAYERS
        .into_iter()
        .map(|layer| (layer, ClaimCeiling::Withheld))
        .collect()
}

pub(super) fn validate(
    ceilings: &BTreeMap<TruthLayer, ClaimCeiling>,
) -> Result<(), ProductFitnessError> {
    if ceilings.len() != TRUTH_LAYERS.len()
        || TRUTH_LAYERS
            .iter()
            .any(|layer| !ceilings.contains_key(layer))
    {
        return Err(ProductFitnessError::MissingTruthLayer);
    }
    let mut lower_proven = true;
    for layer in TRUTH_LAYERS {
        let ceiling = ceilings
            .get(&layer)
            .ok_or(ProductFitnessError::MissingTruthLayer)?;
        if *ceiling == ClaimCeiling::LiveSameSurfaceProven && !lower_proven {
            return Err(ProductFitnessError::TruthLayerEscalation);
        }
        lower_proven &= *ceiling == ClaimCeiling::LiveSameSurfaceProven;
    }
    Ok(())
}
