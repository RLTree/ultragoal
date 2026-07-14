use super::{
    DarwinHostError, DarwinHostErrorId, DarwinHostSurface, DarwinSurfaceObservation,
    DarwinSurfaceStatus, Sha256Digest,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DarwinHostDiagnosis {
    Absent,
    Verified,
    Partial,
    StaleCache,
    Conflict,
    RecoveryPending,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DarwinHostSnapshot {
    installed: DarwinSurfaceObservation,
    cache: DarwinSurfaceObservation,
    marketplace: DarwinSurfaceObservation,
    app_registry: DarwinSurfaceObservation,
    plugins_ui: DarwinSurfaceObservation,
    discovery: DarwinSurfaceObservation,
    runtime: DarwinSurfaceObservation,
    diagnosis: DarwinHostDiagnosis,
    recovery_plan_sha256: Option<Sha256Digest>,
}

impl DarwinHostSnapshot {
    pub(super) fn assemble(
        rows: Vec<DarwinSurfaceObservation>,
        recovery_plan_sha256: Option<String>,
    ) -> Result<Self, DarwinHostError> {
        let mut keyed = BTreeMap::new();
        for row in rows {
            if keyed.insert(row.surface(), row).is_some() {
                return Err(DarwinHostError::new(DarwinHostErrorId::SurfaceConflict));
            }
        }
        if keyed.len() != DarwinHostSurface::ALL.len() {
            return Err(DarwinHostError::new(DarwinHostErrorId::SurfaceConflict));
        }
        let mut take = |surface| {
            keyed
                .remove(&surface)
                .ok_or_else(|| DarwinHostError::at(DarwinHostErrorId::SurfaceConflict, surface))
        };
        let installed = take(DarwinHostSurface::Installed)?;
        let cache = take(DarwinHostSurface::Cache)?;
        let marketplace = take(DarwinHostSurface::Marketplace)?;
        let app_registry = take(DarwinHostSurface::AppRegistry)?;
        let plugins_ui = take(DarwinHostSurface::PluginsUi)?;
        let discovery = take(DarwinHostSurface::Discovery)?;
        let runtime = take(DarwinHostSurface::Runtime)?;
        let recovery_plan_sha256 = recovery_plan_sha256.map(Sha256Digest::parse).transpose()?;
        let diagnosis = diagnose(
            [
                &installed,
                &cache,
                &marketplace,
                &app_registry,
                &plugins_ui,
                &discovery,
                &runtime,
            ],
            recovery_plan_sha256.is_some(),
        );
        Ok(Self {
            installed,
            cache,
            marketplace,
            app_registry,
            plugins_ui,
            discovery,
            runtime,
            diagnosis,
            recovery_plan_sha256,
        })
    }

    pub fn installed(&self) -> &DarwinSurfaceObservation {
        &self.installed
    }
    pub fn cache(&self) -> &DarwinSurfaceObservation {
        &self.cache
    }
    pub fn marketplace(&self) -> &DarwinSurfaceObservation {
        &self.marketplace
    }
    pub fn app_registry(&self) -> &DarwinSurfaceObservation {
        &self.app_registry
    }
    pub fn plugins_ui(&self) -> &DarwinSurfaceObservation {
        &self.plugins_ui
    }
    pub fn discovery(&self) -> &DarwinSurfaceObservation {
        &self.discovery
    }
    pub fn runtime(&self) -> &DarwinSurfaceObservation {
        &self.runtime
    }
    pub const fn diagnosis(&self) -> DarwinHostDiagnosis {
        self.diagnosis
    }
    pub fn recovery_plan_sha256(&self) -> Option<&str> {
        self.recovery_plan_sha256.as_ref().map(Sha256Digest::as_str)
    }

    pub fn surface(&self, surface: DarwinHostSurface) -> &DarwinSurfaceObservation {
        match surface {
            DarwinHostSurface::Installed => self.installed(),
            DarwinHostSurface::Cache => self.cache(),
            DarwinHostSurface::Marketplace => self.marketplace(),
            DarwinHostSurface::AppRegistry => self.app_registry(),
            DarwinHostSurface::PluginsUi => self.plugins_ui(),
            DarwinHostSurface::Discovery => self.discovery(),
            DarwinHostSurface::Runtime => self.runtime(),
        }
    }
}

fn diagnose(rows: [&DarwinSurfaceObservation; 7], recovery: bool) -> DarwinHostDiagnosis {
    if recovery {
        return DarwinHostDiagnosis::RecoveryPending;
    }
    if rows
        .iter()
        .all(|row| row.status() == DarwinSurfaceStatus::Absent)
    {
        return DarwinHostDiagnosis::Absent;
    }
    if rows
        .iter()
        .all(|row| row.status() == DarwinSurfaceStatus::Verified)
    {
        return DarwinHostDiagnosis::Verified;
    }
    if rows
        .iter()
        .enumerate()
        .all(|(index, row)| index == 1 || row.status() == DarwinSurfaceStatus::Verified)
        && matches!(
            rows[1].status(),
            DarwinSurfaceStatus::Stale | DarwinSurfaceStatus::Absent
        )
    {
        return DarwinHostDiagnosis::StaleCache;
    }
    if rows.iter().all(|row| {
        matches!(
            row.status(),
            DarwinSurfaceStatus::Absent | DarwinSurfaceStatus::Verified
        )
    }) {
        DarwinHostDiagnosis::Partial
    } else {
        DarwinHostDiagnosis::Conflict
    }
}
