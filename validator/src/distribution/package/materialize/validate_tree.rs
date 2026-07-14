fn validate_tree(rows: &[TreeObject]) -> Result<(), DistributionError> {
    if rows.len() > TREE_ENTRY_LIMIT {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    let mut paths = BTreeSet::new();
    let mut total = 0usize;
    for row in rows {
        validate_relative_path(&row.path)?;
        total = total
            .checked_add(row.bytes.len())
            .ok_or_else(|| error(DistributionErrorId::ObjectTooLarge))?;
        if total > PACKAGE_LIMIT {
            return Err(error(DistributionErrorId::ObjectTooLarge));
        }
        if row.kind != TreeObjectKind::RegularFile
            || row.link_count != 1
            || !matches!(row.mode, 0o644 | 0o755)
            || !paths.insert(row.path.to_ascii_lowercase())
        {
            return Err(error(DistributionErrorId::UnsafeObject));
        }
    }
    if rows.windows(2).any(|pair| pair[0].path >= pair[1].path) {
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    Ok(())
}

fn read(
    effects: &mut impl MaterializeEffects,
) -> Result<Option<Vec<TreeObject>>, DistributionError> {
    effects
        .read_tree(TREE_ENTRY_LIMIT, PACKAGE_LIMIT)
        .map_err(|_| error(DistributionErrorId::EffectFailed))
}

fn restore(
    effects: &mut impl MaterializeEffects,
    candidate: &str,
    previous: Option<&[TreeObject]>,
) -> Result<(), DistributionError> {
    match effects.compare_exchange_tree(Some(candidate), previous) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(()) => return Err(error(DistributionErrorId::RollbackFailed)),
    }
    let restored = read(effects).map_err(|_| error(DistributionErrorId::RollbackFailed))?;
    if restored.as_deref() != previous {
        return Err(error(DistributionErrorId::RollbackFailed));
    }
    Ok(())
}

fn validate_expected(value: &ExpectedTree) -> Result<(), DistributionError> {
    if matches!(value, ExpectedTree::ExactDigest(row) if !digest(row)) {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(())
}

fn expected_matches(expected: &ExpectedTree, actual: Option<&str>) -> bool {
    match expected {
        ExpectedTree::Absent => actual.is_none(),
        ExpectedTree::ExactDigest(value) => actual == Some(value),
    }
}
