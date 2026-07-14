#[cfg(unix)]
fn install_confined(
    fixture: &crate::package_journey_fixture::JourneyFixture,
    package: &crate::distribution::PackageSnapshot,
    prior: Prior,
) -> crate::distribution::InstallTransaction {
    install_confined_target(fixture, package, TARGET, Scope::PersonalFixture, prior)
}

#[cfg(unix)]
fn install_confined_target(
    fixture: &crate::package_journey_fixture::JourneyFixture,
    package: &crate::distribution::PackageSnapshot,
    target: &str,
    scope: Scope,
    prior: Prior,
) -> crate::distribution::InstallTransaction {
    let plan = InstallPlan::new(
        package.context_id().into(),
        package.candidate_id().into(),
        scope,
        target.into(),
        package.package_sha256().into(),
        prior,
    )
    .unwrap();
    install(
        &plan,
        package,
        &mut ScopedInstall::for_scope(fixture.confined(), scope),
    )
    .unwrap()
}

#[cfg(unix)]
fn inode_tree(root: &std::path::Path) -> Vec<String> {
    let mut rows = Vec::new();
    collect_inode_tree(root, root, &mut rows);
    rows
}

#[cfg(unix)]
fn collect_inode_tree(root: &std::path::Path, path: &std::path::Path, rows: &mut Vec<String>) {
    use std::os::unix::fs::MetadataExt;
    let mut entries = std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path).unwrap();
        rows.push(format!(
            "{}:{}:{}:{}:{}",
            path.strip_prefix(root).unwrap().display(),
            meta.dev(),
            meta.ino(),
            meta.len(),
            meta.mode()
        ));
        if meta.is_dir() {
            collect_inode_tree(root, &path, rows);
        }
    }
}

#[cfg(unix)]
fn refused_transaction(
    failure: RollbackInstallError,
    expected: DistributionErrorId,
) -> crate::distribution::InstallTransaction {
    assert_eq!(failure.id(), expected);
    failure
        .into_transaction()
        .expect("pre-CAS refusal preserves rollback transaction")
}
