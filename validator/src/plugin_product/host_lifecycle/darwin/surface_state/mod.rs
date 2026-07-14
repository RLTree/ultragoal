use super::{DarwinHostError, DarwinHostErrorId, DarwinHostSurface};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DarwinSurfaceStatus {
    Absent,
    Verified,
    Stale,
    Substituted,
    CrossSurface,
    Dirty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DarwinSurfaceObservation {
    surface: DarwinHostSurface,
    state: DarwinSurfaceState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DarwinSurfaceState {
    Absent,
    Verified {
        tree_sha256: Sha256Digest,
        candidate_id: Sha256Digest,
        version: String,
    },
    Conflict {
        status: DarwinSurfaceStatus,
        tree_sha256: Sha256Digest,
        candidate_id: Option<Sha256Digest>,
        version: Option<String>,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in super::super) struct Sha256Digest(String);

impl Sha256Digest {
    pub(super) fn parse(value: impl Into<String>) -> Result<Self, DarwinHostError> {
        let value = value.into();
        if value.len() != 71
            || !value.starts_with("sha256:")
            || !value[7..]
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(DarwinHostError::new(DarwinHostErrorId::SurfaceConflict));
        }
        Ok(Self(value))
    }

    pub(super) fn valid(value: &str) -> bool {
        Self::parse(value).is_ok()
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl DarwinSurfaceObservation {
    pub(super) const fn absent(surface: DarwinHostSurface) -> Self {
        Self {
            surface,
            state: DarwinSurfaceState::Absent,
        }
    }

    pub(super) fn verified(
        surface: DarwinHostSurface,
        tree_sha256: String,
        candidate_id: String,
        version: String,
    ) -> Result<Self, DarwinHostError> {
        if version.trim().is_empty() {
            return Err(DarwinHostError::at(
                DarwinHostErrorId::SurfaceConflict,
                surface,
            ));
        }
        Ok(Self {
            surface,
            state: DarwinSurfaceState::Verified {
                tree_sha256: Sha256Digest::parse(tree_sha256)?,
                candidate_id: Sha256Digest::parse(candidate_id)?,
                version,
            },
        })
    }

    pub(super) fn conflict(
        surface: DarwinHostSurface,
        status: DarwinSurfaceStatus,
        tree_sha256: String,
        candidate_id: Option<String>,
        version: Option<String>,
    ) -> Result<Self, DarwinHostError> {
        if matches!(
            status,
            DarwinSurfaceStatus::Absent | DarwinSurfaceStatus::Verified
        ) || version.as_deref().is_some_and(str::is_empty)
        {
            return Err(DarwinHostError::at(
                DarwinHostErrorId::SurfaceConflict,
                surface,
            ));
        }
        Ok(Self {
            surface,
            state: DarwinSurfaceState::Conflict {
                status,
                tree_sha256: Sha256Digest::parse(tree_sha256)?,
                candidate_id: candidate_id.map(Sha256Digest::parse).transpose()?,
                version,
            },
        })
    }

    pub const fn surface(&self) -> DarwinHostSurface {
        self.surface
    }

    pub const fn status(&self) -> DarwinSurfaceStatus {
        match &self.state {
            DarwinSurfaceState::Absent => DarwinSurfaceStatus::Absent,
            DarwinSurfaceState::Verified { .. } => DarwinSurfaceStatus::Verified,
            DarwinSurfaceState::Conflict { status, .. } => *status,
        }
    }

    pub fn tree_sha256(&self) -> Option<&str> {
        match &self.state {
            DarwinSurfaceState::Absent => None,
            DarwinSurfaceState::Verified { tree_sha256, .. }
            | DarwinSurfaceState::Conflict { tree_sha256, .. } => Some(tree_sha256.as_str()),
        }
    }

    pub fn observed_candidate_id(&self) -> Option<&str> {
        match &self.state {
            DarwinSurfaceState::Verified { candidate_id, .. } => Some(candidate_id.as_str()),
            DarwinSurfaceState::Conflict { candidate_id, .. } => {
                candidate_id.as_ref().map(Sha256Digest::as_str)
            }
            DarwinSurfaceState::Absent => None,
        }
    }

    pub fn observed_version(&self) -> Option<&str> {
        match &self.state {
            DarwinSurfaceState::Verified { version, .. } => Some(version),
            DarwinSurfaceState::Conflict { version, .. } => version.as_deref(),
            DarwinSurfaceState::Absent => None,
        }
    }
}
