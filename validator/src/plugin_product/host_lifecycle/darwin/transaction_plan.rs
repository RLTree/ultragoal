use super::surface_codec::{PackageBinding, encode_surface, surface_tree_sha256};
use super::transaction_codec::{
    DarwinTransactionCodecRequest, DarwinTransactionCodecResponse, execute,
};
use super::{
    DarwinHostError, DarwinHostErrorId, DarwinHostOperation, DarwinHostSurface,
    SUPPORTED_MARKETPLACE, SUPPORTED_PLUGIN_ID, SUPPORTED_VERSION,
};
use crate::distribution::PackageSnapshot;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(super) struct SurfaceStep {
    pub(super) surface: DarwinHostSurface,
    pub(super) desired: Option<Vec<u8>>,
    pub(super) desired_tree_sha256: Option<String>,
    pub(super) prior_tree_sha256: Option<String>,
}

#[derive(Clone, Debug)]
pub struct DarwinHostTransactionPlan {
    pub(super) root_id: String,
    pub(super) operation: DarwinHostOperation,
    pub(super) target: PackageBinding,
    pub(super) prior: Option<PackageBinding>,
    pub(super) marketplace: String,
    pub(super) steps: Vec<SurfaceStep>,
    pub(super) desired_by_surface: BTreeMap<DarwinHostSurface, Vec<u8>>,
    pub(super) prior_by_surface: BTreeMap<DarwinHostSurface, Vec<u8>>,
    pub(super) plan_sha256: String,
}

impl DarwinHostTransactionPlan {
    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub const fn operation(&self) -> DarwinHostOperation {
        self.operation
    }

    pub fn candidate_id(&self) -> &str {
        &self.target.candidate_id
    }

    pub fn version(&self) -> &str {
        &self.target.version
    }

    pub fn marketplace(&self) -> &str {
        &self.marketplace
    }

    pub(super) fn build(
        root_id: &str,
        operation: DarwinHostOperation,
        target_snapshot: &PackageSnapshot,
        prior_snapshot: Option<&PackageSnapshot>,
        marketplace: &str,
    ) -> Result<Self, DarwinHostError> {
        if marketplace != SUPPORTED_MARKETPLACE {
            return Err(DarwinHostError::new(DarwinHostErrorId::InvalidMarketplace));
        }
        let target = PackageBinding::from_snapshot(target_snapshot)?;
        if target.plugin_id != SUPPORTED_PLUGIN_ID {
            return Err(DarwinHostError::new(DarwinHostErrorId::InvalidPackage));
        }
        if target.version != SUPPORTED_VERSION {
            return Err(DarwinHostError::new(DarwinHostErrorId::UnsupportedVersion));
        }
        let prior = prior_snapshot
            .map(PackageBinding::from_snapshot)
            .transpose()?;
        validate_operation(operation, &target, prior.as_ref())?;
        let mut desired_by_surface = BTreeMap::new();
        for surface in DarwinHostSurface::ALL {
            desired_by_surface.insert(
                surface,
                encode_surface(surface, root_id, &target, target_snapshot.archive())?,
            );
        }
        let mut prior_by_surface = BTreeMap::new();
        if let (Some(prior), Some(snapshot)) = (&prior, prior_snapshot) {
            for surface in DarwinHostSurface::ALL {
                prior_by_surface.insert(
                    surface,
                    encode_surface(surface, root_id, prior, snapshot.archive())?,
                );
            }
        }
        let surfaces: &[DarwinHostSurface] = match operation {
            DarwinHostOperation::RepairCache => &[DarwinHostSurface::Cache],
            _ => &DarwinHostSurface::ALL,
        };
        let mut steps = Vec::with_capacity(surfaces.len());
        for &surface in surfaces {
            let desired = if operation == DarwinHostOperation::Uninstall {
                None
            } else {
                Some(desired_by_surface[&surface].clone())
            };
            let desired_tree_sha256 = desired.as_deref().map(surface_tree_sha256).transpose()?;
            let prior_tree_sha256 = prior_by_surface
                .get(&surface)
                .map(|bytes| surface_tree_sha256(bytes))
                .transpose()?;
            steps.push(SurfaceStep {
                surface,
                desired,
                desired_tree_sha256,
                prior_tree_sha256,
            });
        }
        let plan_sha256 = plan_sha256_from_parts(
            root_id,
            operation,
            &target,
            &prior,
            marketplace,
            steps
                .iter()
                .map(|row| {
                    (
                        row.surface,
                        row.desired_tree_sha256.as_deref(),
                        row.prior_tree_sha256.as_deref(),
                    )
                })
                .collect(),
        )?;
        Ok(Self {
            root_id: root_id.to_owned(),
            operation,
            target,
            prior,
            marketplace: marketplace.to_owned(),
            steps,
            desired_by_surface,
            prior_by_surface,
            plan_sha256,
        })
    }
}

pub(super) fn plan_sha256_from_parts<'a>(
    root_id: &'a str,
    operation: DarwinHostOperation,
    target: &'a PackageBinding,
    prior: &'a Option<PackageBinding>,
    marketplace: &'a str,
    steps: Vec<(DarwinHostSurface, Option<&'a str>, Option<&'a str>)>,
) -> Result<String, DarwinHostError> {
    let response = execute(DarwinTransactionCodecRequest::PlanDigest {
        root_id,
        operation,
        target,
        prior,
        marketplace,
        steps: &steps,
    })
    .map_err(|_| DarwinHostError::new(DarwinHostErrorId::InvalidOperation))?;
    match response {
        DarwinTransactionCodecResponse::Digest(value) => Ok(value),
        _ => Err(DarwinHostError::new(DarwinHostErrorId::InvalidOperation)),
    }
}

pub(super) fn validate_operation(
    operation: DarwinHostOperation,
    target: &PackageBinding,
    prior: Option<&PackageBinding>,
) -> Result<(), DarwinHostError> {
    match operation {
        DarwinHostOperation::Install
        | DarwinHostOperation::Reinstall
        | DarwinHostOperation::Uninstall
            if prior.is_none() =>
        {
            Ok(())
        }
        DarwinHostOperation::Update | DarwinHostOperation::RepairCache => {
            let prior =
                prior.ok_or_else(|| DarwinHostError::new(DarwinHostErrorId::InvalidOperation))?;
            if prior.plugin_id != target.plugin_id
                || version_tuple(&prior.version)? >= version_tuple(&target.version)?
            {
                return Err(DarwinHostError::new(DarwinHostErrorId::InvalidOperation));
            }
            Ok(())
        }
        _ => Err(DarwinHostError::new(DarwinHostErrorId::InvalidOperation)),
    }
}

fn version_tuple(value: &str) -> Result<(u64, u64, u64), DarwinHostError> {
    let rows = value
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| DarwinHostError::new(DarwinHostErrorId::InvalidPackage))?;
    if rows.len() != 3 {
        return Err(DarwinHostError::new(DarwinHostErrorId::InvalidPackage));
    }
    Ok((rows[0], rows[1], rows[2]))
}
