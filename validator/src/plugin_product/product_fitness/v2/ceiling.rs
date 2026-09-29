use super::super::model::{
    ClaimCeiling, EvidenceClass, PRODUCT_FITNESS_SURFACES, ProductFitnessDisposition,
    ProductFitnessError, SurfaceIdentities, SurfaceStatus, TruthLayer,
};

pub(super) fn validate(
    disposition: &ProductFitnessDisposition,
    class: EvidenceClass,
    surfaces: &SurfaceIdentities,
) -> Result<(), ProductFitnessError> {
    for layer in PRODUCT_FITNESS_SURFACES {
        let ceiling = disposition
            .truth_layer_ceilings
            .get(&layer)
            .ok_or(ProductFitnessError::MissingTruthLayer)?;
        if *ceiling != ClaimCeiling::LiveSameSurfaceProven {
            continue;
        }
        if !supports_live_layer(layer, class) {
            return Err(ProductFitnessError::UnsupportedClaimCeiling);
        }
        if surface_status(layer, surfaces) != SurfaceStatus::Observed {
            return Err(ProductFitnessError::MissingObservation);
        }
        if class == EvidenceClass::AgentUse && layer == TruthLayer::Journey {
            return Err(ProductFitnessError::UnsupportedClaimCeiling);
        }
    }
    Ok(())
}

fn surface_status(layer: TruthLayer, surfaces: &SurfaceIdentities) -> SurfaceStatus {
    match layer {
        TruthLayer::Source => surfaces.source.status,
        TruthLayer::Package => surfaces.package.status,
        TruthLayer::Marketplace => surfaces.marketplace.status,
        TruthLayer::Install => surfaces.install.status,
        TruthLayer::Cache => surfaces.cache.status,
        TruthLayer::AppRegistry => surfaces.app_registry.status,
        TruthLayer::Discovery => surfaces.discovery.status,
        TruthLayer::Runtime => surfaces.runtime.status,
        TruthLayer::Journey => surfaces.journey.status,
        TruthLayer::PluginsUi => SurfaceStatus::Withheld,
    }
}

fn supports_live_layer(layer: TruthLayer, class: EvidenceClass) -> bool {
    match class {
        EvidenceClass::Intent | EvidenceClass::Research | EvidenceClass::Prototype => false,
        EvidenceClass::Source => layer == TruthLayer::Source,
        EvidenceClass::Package => matches!(layer, TruthLayer::Source | TruthLayer::Package),
        EvidenceClass::Installed => matches!(
            layer,
            TruthLayer::Source
                | TruthLayer::Package
                | TruthLayer::Marketplace
                | TruthLayer::Install
        ),
        EvidenceClass::Runtime | EvidenceClass::AgentUse => matches!(
            layer,
            TruthLayer::Source
                | TruthLayer::Package
                | TruthLayer::Marketplace
                | TruthLayer::Install
                | TruthLayer::Cache
                | TruthLayer::AppRegistry
                | TruthLayer::Discovery
                | TruthLayer::Runtime
        ),
        EvidenceClass::HumanUse | EvidenceClass::RepeatedHumanUse => {
            PRODUCT_FITNESS_SURFACES.contains(&layer)
        }
    }
}
