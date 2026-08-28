const COMMAND_TIMEOUT_MS: u64 = 30_000;
const COMMAND_MAX_ATTEMPTS: u8 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HostCommand {
    program: String,
    argv: Vec<String>,
    environment: Vec<(String, String)>,
    timeout_ms: u64,
    max_attempts: u8,
}

impl HostCommand {
    pub(crate) fn from_untrusted_record(
        program: String,
        argv: Vec<String>,
        environment: Vec<(String, String)>,
        timeout_ms: u64,
        max_attempts: u8,
    ) -> Self {
        Self {
            program,
            argv,
            environment,
            timeout_ms,
            max_attempts,
        }
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn argv(&self) -> &[String] {
        &self.argv
    }

    pub fn environment(&self) -> &[(String, String)] {
        &self.environment
    }

    pub const fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    pub const fn max_attempts(&self) -> u8 {
        self.max_attempts
    }
}

pub struct HostCommandPlan {
    package: PackageIdentity,
    commands: Vec<HostCommand>,
    plan_sha256: String,
}

impl Clone for HostCommandPlan {
    fn clone(&self) -> Self {
        Self {
            package: self.package.clone(),
            commands: self.commands.clone(),
            plan_sha256: self.plan_sha256.clone(),
        }
    }
}

impl PartialEq for HostCommandPlan {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package
            && self.commands == other.commands
            && self.plan_sha256 == other.plan_sha256
    }
}

impl Eq for HostCommandPlan {}

impl std::fmt::Debug for HostCommandPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostCommandPlan")
            .field("package", &self.package)
            .field("command_count", &self.commands.len())
            .field("plan_sha256", &self.plan_sha256)
            .finish()
    }
}

impl HostCommandPlan {
    pub fn repository_install(
        package: &PackageIdentity,
        repository_root: &str,
        repository_marketplace: &str,
    ) -> Result<Self, DistributionError> {
        validate_path_argument(repository_root)?;
        validate_name(repository_marketplace)?;
        let commands = vec![
            command(&["plugin", "marketplace", "add", repository_root]),
            command(&[
                "plugin",
                "add",
                &format!("harness-ultragoal@{repository_marketplace}"),
            ]),
        ];
        bound_plan(package, commands)
    }

    pub fn personal_install(
        package: &PackageIdentity,
        marketplace: &str,
    ) -> Result<Self, DistributionError> {
        host_plugin_plan(package, "add", marketplace)
    }

    pub(crate) fn personal_install_in_codex_home(
        package: &PackageIdentity,
        marketplace: &str,
        home: &Path,
    ) -> Result<Self, DistributionError> {
        validate_name(marketplace)?;
        let requested_home = home;
        let home = requested_home
            .canonicalize()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if home != requested_home || !home.is_absolute() {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        let codex_home = home
            .join(".codex")
            .canonicalize()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if codex_home != home.join(".codex") {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        let home = home.display().to_string();
        let codex_home = codex_home.display().to_string();
        bound_plan(
            package,
            vec![command_with_environment(
                &["plugin", "add", &format!("harness-ultragoal@{marketplace}")],
                &[
                    ("CODEX_HOME".to_owned(), codex_home),
                    ("HOME".to_owned(), home),
                ],
            )],
        )
    }

    pub fn personal_remove(
        package: &PackageIdentity,
        marketplace: &str,
    ) -> Result<Self, DistributionError> {
        host_plugin_plan(package, "remove", marketplace)
    }

    pub fn repository_remove(
        package: &PackageIdentity,
        marketplace: &str,
    ) -> Result<Self, DistributionError> {
        validate_name(marketplace)?;
        let commands = vec![
            command(&[
                "plugin",
                "remove",
                &format!("harness-ultragoal@{marketplace}"),
            ]),
            command(&["plugin", "marketplace", "remove", marketplace]),
        ];
        bound_plan(package, commands)
    }

    pub fn commands(&self) -> &[HostCommand] {
        &self.commands
    }

    pub(crate) fn package(&self) -> &PackageIdentity {
        &self.package
    }

    pub(crate) fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }
}

fn host_plugin_plan(
    package: &PackageIdentity,
    action: &str,
    marketplace: &str,
) -> Result<HostCommandPlan, DistributionError> {
    validate_name(marketplace)?;
    let commands = vec![command(&[
        "plugin",
        action,
        &format!("harness-ultragoal@{marketplace}"),
    ])];
    bound_plan(package, commands)
}

fn command(argv: &[&str]) -> HostCommand {
    command_with_environment(argv, &[])
}

fn command_with_environment(argv: &[&str], environment: &[(String, String)]) -> HostCommand {
    HostCommand {
        program: "codex".to_owned(),
        argv: argv.iter().map(|row| (*row).to_owned()).collect(),
        environment: environment.to_vec(),
        timeout_ms: COMMAND_TIMEOUT_MS,
        max_attempts: COMMAND_MAX_ATTEMPTS,
    }
}

fn bound_plan(
    package: &PackageIdentity,
    commands: Vec<HostCommand>,
) -> Result<HostCommandPlan, DistributionError> {
    package.validate()?;
    #[derive(Serialize)]
    struct Binding<'a> {
        schema: &'static str,
        package: &'a PackageIdentity,
        commands: &'a [HostCommand],
    }
    let binding = Binding {
        schema: "harness-ultragoal.host-command-plan.v1",
        package,
        commands: &commands,
    };
    let plan_sha256 = serde_json::to_vec(&binding)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| error(DistributionErrorId::InvalidSpec))?;
    Ok(HostCommandPlan {
        package: package.clone(),
        commands,
        plan_sha256,
    })
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

fn validate_path_argument(value: &str) -> Result<(), DistributionError> {
    use std::path::Component;
    let mut components = std::path::Path::new(value).components();
    let shape = matches!(components.next(), Some(Component::RootDir))
        && components.all(|row| matches!(row, Component::Normal(_)));
    if value.is_empty()
        || value.len() > 4096
        || value
            .bytes()
            .any(|byte| byte == 0 || byte.is_ascii_control())
        || !shape
    {
        return Err(error(DistributionErrorId::InvalidPath));
    }
    Ok(())
}

#[cfg(test)]
mod personal_install_tests {
    use super::*;
    use crate::distribution::{PackageIdentity, SourceIdentity};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_HOME: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn personal_install_binds_user_home_and_global_codex_root_separately() {
        let home = std::env::temp_dir().join(format!(
            "hul-personal-command-home-{}-{}",
            std::process::id(),
            NEXT_HOME.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        let home = home.canonicalize().unwrap();
        let plan = HostCommandPlan::personal_install_in_codex_home(
            &package(),
            "local-harness-plugins",
            &home,
        )
        .unwrap();
        let command = &plan.commands()[0];
        assert_eq!(command.program(), "codex");
        assert_eq!(
            command.argv(),
            [
                "plugin".to_owned(),
                "add".to_owned(),
                "harness-ultragoal@local-harness-plugins".to_owned(),
            ]
        );
        assert_eq!(
            command.environment(),
            [
                (
                    "CODEX_HOME".to_owned(),
                    home.join(".codex").display().to_string(),
                ),
                ("HOME".to_owned(), home.display().to_string()),
            ]
        );
        assert_ne!(command.environment()[0].1, command.environment()[1].1);
        std::fs::remove_dir_all(home).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn personal_install_rejects_an_aliased_home() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "hul-personal-command-alias-{}-{}",
            std::process::id(),
            NEXT_HOME.fetch_add(1, Ordering::Relaxed)
        ));
        let home = root.join("home");
        let alias = root.join("alias");
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        symlink(&home, &alias).unwrap();
        assert!(
            HostCommandPlan::personal_install_in_codex_home(
                &package(),
                "local-harness-plugins",
                &alias,
            )
            .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    fn package() -> PackageIdentity {
        let digest = |seed: char| format!("sha256:{}", seed.to_string().repeat(64));
        PackageIdentity::new(
            SourceIdentity::new(
                digest('1'),
                digest('2'),
                "harness-ultragoal".to_owned(),
                "0.0.42".to_owned(),
                digest('3'),
                digest('4'),
            )
            .unwrap(),
            digest('5'),
            digest('6'),
        )
        .unwrap()
    }
}
