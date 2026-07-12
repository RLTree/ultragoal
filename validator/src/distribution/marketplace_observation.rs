use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::marketplace::{
    MarketplaceExpectation, MarketplacePlan, MarketplaceScope, MarketplaceSnapshot,
    MarketplaceVerdict, snapshot,
};
use crate::distribution::reader::sha256;

pub fn observe_codex_marketplace(
    bytes: &[u8],
    plan: &MarketplacePlan,
    scope: MarketplaceScope,
) -> Result<MarketplaceSnapshot, DistributionError> {
    if bytes != plan.replacement() {
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    let package = plan.package();
    let source = package.source();
    let expected = MarketplaceExpectation::new(
        source.context_id().into(),
        source.candidate_id().into(),
        scope,
        source.plugin_id().into(),
        source.version().into(),
        "plugins/harness-ultragoal".into(),
        package.archive_sha256().into(),
    )?;
    Ok(snapshot(
        &expected,
        MarketplaceVerdict::Verified,
        Some(sha256(bytes)),
    ))
}
