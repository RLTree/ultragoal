use super::effect_request::HostScopeAuthority;
use super::error::{HostLifecycleError, HostLifecycleErrorId};
use crate::distribution::{
    CacheExpectation, HostCapabilityDeclaration, MarketplacePlan, MarketplaceScope, PackageIdentity,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;

pub(super) struct BoundHostScope {
    authority: HostScopeAuthority,
    host: HostCapabilityDeclaration,
    marketplace_scope: MarketplaceScope,
    cache: CacheExpectation,
    scope_sha256: String,
}

impl BoundHostScope {
    pub(super) fn bind(
        package: &PackageIdentity,
        host: &HostCapabilityDeclaration,
        marketplace_plan: &MarketplacePlan,
        authority: HostScopeAuthority,
    ) -> Result<Arc<Self>, HostLifecycleError> {
        if marketplace_plan.package() != package {
            return Err(invalid());
        }
        validate_marketplace_plan(marketplace_plan, &authority)?;
        validate_repository_scope(host, &authority)?;
        let source = package.source();
        let cache = CacheExpectation::new(
            source.context_id().to_owned(),
            source.candidate_id().to_owned(),
            host.home_id().to_owned(),
            authority.marketplace().to_owned(),
            source.plugin_id().to_owned(),
            source.version().to_owned(),
            package.tree_sha256().to_owned(),
        )
        .map_err(|_| invalid())?;
        let marketplace_scope = authority.marketplace_scope();
        let scope_sha256 = scope_digest(package, host, marketplace_plan, &authority)?;
        Ok(Arc::new(Self {
            authority,
            host: host.clone(),
            marketplace_scope,
            cache,
            scope_sha256,
        }))
    }

    pub(super) fn authority(&self) -> &HostScopeAuthority {
        &self.authority
    }

    pub(super) fn host(&self) -> &HostCapabilityDeclaration {
        &self.host
    }

    pub(super) fn marketplace_scope(&self) -> MarketplaceScope {
        self.marketplace_scope
    }

    pub(super) fn cache(&self) -> &CacheExpectation {
        &self.cache
    }

    pub(super) fn scope_sha256(&self) -> &str {
        &self.scope_sha256
    }

    pub(super) fn revalidate(&self) -> Result<(), HostLifecycleError> {
        validate_repository_scope(&self.host, &self.authority)
    }
}

pub(super) fn same_scope(left: &Arc<BoundHostScope>, right: &Arc<BoundHostScope>) -> bool {
    Arc::ptr_eq(left, right)
        && left.scope_sha256 == right.scope_sha256
        && left.authority == right.authority
        && left.host == right.host
        && left.marketplace_scope == right.marketplace_scope
}

fn validate_marketplace_plan(
    supplied: &MarketplacePlan,
    authority: &HostScopeAuthority,
) -> Result<(), HostLifecycleError> {
    #[derive(Deserialize)]
    struct MarketplaceName {
        name: String,
    }
    let observed: MarketplaceName =
        serde_json::from_slice(supplied.replacement()).map_err(|_| invalid())?;
    if observed.name != authority.marketplace() {
        return Err(invalid());
    }
    Ok(())
}

fn validate_repository_scope(
    host: &HostCapabilityDeclaration,
    authority: &HostScopeAuthority,
) -> Result<(), HostLifecycleError> {
    let HostScopeAuthority::Repository {
        repository_root, ..
    } = authority
    else {
        return Ok(());
    };
    let observed = HostCapabilityDeclaration::isolated(
        Path::new(repository_root),
        Path::new(repository_root),
        "host-lifecycle-scope-validation-v1",
        None,
    )
    .map_err(|_| HostLifecycleError::new(HostLifecycleErrorId::HostScopeRejected))?;
    if observed.project_id() != host.project_id() {
        return Err(HostLifecycleError::new(
            HostLifecycleErrorId::HostScopeRejected,
        ));
    }
    Ok(())
}

fn scope_digest(
    package: &PackageIdentity,
    host: &HostCapabilityDeclaration,
    marketplace_plan: &MarketplacePlan,
    authority: &HostScopeAuthority,
) -> Result<String, HostLifecycleError> {
    #[derive(Serialize)]
    struct Scope<'a> {
        schema: &'static str,
        package: &'a PackageIdentity,
        host_capability_sha256: &'a str,
        authority: &'a HostScopeAuthority,
        marketplace_scope: MarketplaceScope,
        marketplace_replacement_sha256: String,
    }
    let bytes = serde_json::to_vec(&Scope {
        schema: "harness-ultragoal.bound-host-scope.v1",
        package,
        host_capability_sha256: host.capability_sha256(),
        authority,
        marketplace_scope: authority.marketplace_scope(),
        marketplace_replacement_sha256: format!(
            "sha256:{:x}",
            Sha256::digest(marketplace_plan.replacement())
        ),
    })
    .map_err(|_| invalid())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn invalid() -> HostLifecycleError {
    HostLifecycleError::new(HostLifecycleErrorId::InvalidBinding)
}
