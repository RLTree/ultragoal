#[derive(Clone, Copy)]
pub(super) enum SurfaceOperation {
    InstallAudit,
    CacheAudit,
}

impl SurfaceOperation {
    pub(super) fn id(self) -> &'static str {
        match self {
            Self::InstallAudit => "install_audit",
            Self::CacheAudit => "cache_audit",
        }
    }

    pub(super) fn surface_id(self) -> &'static str {
        match self {
            Self::InstallAudit => "installed_plugin",
            Self::CacheAudit => "versioned_cache_package",
        }
    }
}
