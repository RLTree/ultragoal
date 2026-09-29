use super::model::{
    ClaimCeiling, PRODUCT_FITNESS_SURFACES, ProductFitnessError, TRUTH_LAYERS, TruthLayer,
};
use std::collections::BTreeMap;

pub fn source_candidate_ceilings() -> BTreeMap<TruthLayer, ClaimCeiling> {
    TRUTH_LAYERS
        .into_iter()
        .map(|layer| (layer, ClaimCeiling::Withheld))
        .collect()
}

pub fn product_fitness_candidate_ceilings() -> BTreeMap<TruthLayer, ClaimCeiling> {
    PRODUCT_FITNESS_SURFACES
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

pub(super) fn validate_product_fitness(
    ceilings: &BTreeMap<TruthLayer, ClaimCeiling>,
    claimed: TruthLayer,
) -> Result<(), ProductFitnessError> {
    if claimed == TruthLayer::PluginsUi
        || ceilings.len() != PRODUCT_FITNESS_SURFACES.len()
        || PRODUCT_FITNESS_SURFACES
            .iter()
            .any(|layer| !ceilings.contains_key(layer))
    {
        return Err(ProductFitnessError::MissingTruthLayer);
    }
    for layer in PRODUCT_FITNESS_SURFACES {
        if ceilings[&layer] != ClaimCeiling::LiveSameSurfaceProven {
            continue;
        }
        if predecessors(layer)
            .iter()
            .any(|predecessor| ceilings[predecessor] != ClaimCeiling::LiveSameSurfaceProven)
        {
            return Err(ProductFitnessError::TruthLayerEscalation);
        }
    }
    Ok(())
}

fn predecessors(layer: TruthLayer) -> &'static [TruthLayer] {
    use TruthLayer::*;
    match layer {
        Source => &[],
        Package => &[Source],
        Marketplace => &[Source, Package],
        Install => &[Source, Package],
        Cache => &[Source, Package, Install],
        AppRegistry => &[Source, Package, Install],
        Discovery => &[Source, Package, Install],
        Runtime => &[Source, Package, Install, Discovery],
        Journey => &[Source, Package, Install, Discovery, Runtime],
        PluginsUi => &[],
    }
}
