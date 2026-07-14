#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyCatalog {
    schema: String,
    context_id: String,
    candidate_id: String,
    scope: crate::distribution::marketplace::MarketplaceScope,
    plugins: Vec<LegacyPlugin>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyPlugin {
    plugin_id: String,
    version: String,
    origin: String,
    package_sha256: String,
}

pub fn verify_marketplace(
    bytes: &[u8],
    expected: &MarketplaceExpectation,
) -> Result<MarketplaceSnapshot, DistributionError> {
    let catalog: LegacyCatalog = json::parse(bytes, 1024 * 1024)?;
    if catalog.schema != "harness-ultragoal.marketplace-catalog.v1"
        || catalog.context_id != expected.context_id
        || catalog.candidate_id != expected.candidate_id
        || catalog.scope != expected.scope
        || catalog.plugins.is_empty()
        || catalog.plugins.len() > 1024
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    let mut seen = BTreeSet::new();
    let mut matching = Vec::new();
    for row in catalog.plugins {
        validate_relative_path(&row.origin)?;
        if Version::parse(&row.version).is_none()
            || !digest(&row.package_sha256)
            || !seen.insert((row.plugin_id.clone(), row.version.clone()))
        {
            return Err(error(DistributionErrorId::InstallConflict));
        }
        if row.plugin_id == expected.plugin_id {
            matching.push(row);
        }
    }
    if matching.len() != 1
        || matching[0].version != expected.version
        || matching[0].origin != expected.origin
        || matching[0].package_sha256 != expected.package_sha256
    {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    Ok(snapshot(
        expected,
        MarketplaceVerdict::Verified,
        Some(sha256(bytes)),
    ))
}

pub fn unavailable_marketplace(
    expected: &MarketplaceExpectation,
    reason_id: &str,
) -> Result<MarketplaceSnapshot, DistributionError> {
    if reason_id.is_empty()
        || reason_id.len() > 96
        || !reason_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(snapshot(expected, MarketplaceVerdict::Unavailable, None))
}
