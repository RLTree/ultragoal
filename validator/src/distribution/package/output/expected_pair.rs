fn expected_pair(snapshot: &PackageSnapshot) -> Result<Vec<TreeObject>, DistributionError> {
    let source = snapshot.identity().source();
    let prefix = format!(
        "{}-{}",
        source.plugin_id(),
        output_filename_version(source.version())
    );
    let mut rows = vec![
        TreeObject::regular(
            format!("{prefix}.hugpkg"),
            0o644,
            snapshot.archive().to_vec(),
        ),
        TreeObject::regular(
            format!("{prefix}.inventory.json"),
            0o644,
            snapshot.inventory().to_vec(),
        ),
    ];
    rows.sort_by(|left, right| left.path().cmp(right.path()));
    tree_sha256(&rows)?;
    Ok(rows)
}

pub(crate) fn output_filename_version(version: &str) -> String {
    version.replace('+', "_plus_")
}

fn collides(previous: Option<&[TreeObject]>, replacement: &[TreeObject]) -> bool {
    previous.is_some_and(|rows| {
        rows.iter().any(|existing| {
            replacement.iter().any(|candidate| {
                existing.path() == candidate.path() && existing.bytes() != candidate.bytes()
            })
        })
    })
}

fn is_complete_artifact_pair(rows: &[TreeObject]) -> bool {
    if rows.len() != OUTPUT_ENTRY_LIMIT {
        return false;
    }
    let package = rows
        .iter()
        .find_map(|row| row.path().strip_suffix(".hugpkg"));
    let inventory = rows
        .iter()
        .find_map(|row| row.path().strip_suffix(".inventory.json"));
    matches!((package, inventory), (Some(left), Some(right)) if !left.is_empty() && left == right)
}

fn read(
    effects: &mut impl MaterializeEffects,
) -> Result<Option<Vec<TreeObject>>, DistributionError> {
    effects
        .read_tree(OUTPUT_ENTRY_LIMIT, OUTPUT_BYTE_LIMIT)
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
        Err(_) => return Err(error(DistributionErrorId::RollbackFailed)),
    }
    let restored = read(effects).map_err(|_| error(DistributionErrorId::RollbackFailed))?;
    if restored.as_deref() != previous {
        return Err(error(DistributionErrorId::RollbackFailed));
    }
    Ok(())
}

fn expected_matches(expected: &ExpectedTree, actual: Option<&str>) -> bool {
    match expected {
        ExpectedTree::Absent => actual.is_none(),
        ExpectedTree::ExactDigest(value) => actual == Some(value),
    }
}

#[cfg(test)]
mod filename_version_tests {
    use super::output_filename_version;

    #[test]
    fn semver_build_separator_has_one_injective_path_safe_encoding() {
        assert_eq!(
            output_filename_version("0.0.42+codex.20260828085546"),
            "0.0.41_plus_codex.20260824093100"
        );
        assert_ne!(
            output_filename_version("0.0.41"),
            output_filename_version("0.0.42+codex.20260828085546")
        );
        assert_ne!(
            output_filename_version("0.0.41+codex.first"),
            output_filename_version("0.0.41+codex.second")
        );
    }
}
