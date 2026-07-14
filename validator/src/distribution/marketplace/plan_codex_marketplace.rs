pub fn plan_codex_marketplace(
    current: Option<&[u8]>,
    expected_sha256: Option<&str>,
    name: &str,
    display_name: &str,
    desired: CodexPlugin,
    package: PackageIdentity,
) -> Result<MarketplacePlan, DistributionError> {
    package.validate()?;
    validate_name(name)?;
    if display_name.is_empty()
        || display_name.len() > 128
        || display_name.bytes().any(|byte| byte.is_ascii_control())
        || expected_sha256.is_some_and(|value| !digest(value))
        || current.map(sha256).as_deref() != expected_sha256
    {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    validate_plugin(&desired)?;
    let mut document = match current {
        Some(bytes) => json::parse::<CodexMarketplace>(bytes, MARKETPLACE_LIMIT)?,
        None => CodexMarketplace {
            name: name.into(),
            interface: CodexInterface {
                display_name: display_name.into(),
            },
            plugins: Vec::new(),
        },
    };
    validate_document(&document)?;
    if document.name != name {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    match document
        .plugins
        .iter()
        .position(|row| row.name == desired.name)
    {
        Some(index) => document.plugins[index] = desired,
        None => document.plugins.push(desired),
    }
    validate_document(&document)?;
    let mut replacement = serde_json::to_vec_pretty(&document)
        .map_err(|_| error(DistributionErrorId::InvalidSpec))?;
    replacement.push(b'\n');
    Ok(MarketplacePlan {
        expected_sha256: expected_sha256.map(str::to_owned),
        replacement,
        rollback: current.map(<[u8]>::to_vec),
        package,
    })
}

fn validate_document(value: &CodexMarketplace) -> Result<(), DistributionError> {
    validate_name(&value.name)?;
    if value.interface.display_name.is_empty()
        || value.interface.display_name.len() > 128
        || value
            .interface
            .display_name
            .bytes()
            .any(|byte| byte.is_ascii_control())
        || value.plugins.len() > 1024
    {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    let mut names = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for row in &value.plugins {
        validate_plugin(row)?;
        if !names.insert(&row.name) || !paths.insert(&row.source.path) {
            return Err(error(DistributionErrorId::InstallConflict));
        }
    }
    Ok(())
}

fn validate_plugin(value: &CodexPlugin) -> Result<(), DistributionError> {
    validate_name(&value.name)?;
    let path = value
        .source
        .path
        .strip_prefix("./")
        .ok_or_else(|| error(DistributionErrorId::InvalidPath))?;
    validate_relative_path(path)?;
    if value.source.source != "local"
        || value.policy.installation != "AVAILABLE"
        || value.policy.authentication != "ON_INSTALL"
        || value.category.is_empty()
        || value.category.len() > 128
        || value.category.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(())
}

fn validate_name(value: &str) -> Result<(), DistributionError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(())
}
