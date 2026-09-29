impl HostCommandPlan {
    pub(crate) fn repository_install_in_isolated_codex_home(
        package: &PackageIdentity,
        repository_root: &str,
        repository_marketplace: &str,
        isolated_home: &Path,
    ) -> Result<Self, DistributionError> {
        validate_path_argument(repository_root)?;
        validate_name(repository_marketplace)?;
        let home = isolated_home
            .canonicalize()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if !home.is_absolute() {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        let environment = vec![
            ("CODEX_HOME".to_owned(), home.display().to_string()),
            ("HOME".to_owned(), home.display().to_string()),
        ];
        let commands = vec![
            command_with_environment(
                &["plugin", "marketplace", "add", repository_root],
                &environment,
            ),
            command_with_environment(
                &[
                    "plugin",
                    "add",
                    &format!("harness-ultragoal@{repository_marketplace}"),
                ],
                &environment,
            ),
        ];
        bound_plan(package, commands)
    }
}
