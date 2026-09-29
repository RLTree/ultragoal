use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RoutineInvocationOptions {
    target: Option<String>,
    interruption: Option<ReservationInterruption>,
    continuation: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReservationInterruption {
    AfterReservation,
}

impl RoutineInvocationOptions {
    pub(crate) fn interruption(&self) -> Option<ReservationInterruption> {
        self.interruption
    }

    pub(crate) fn continuation(&self) -> Option<&str> {
        self.continuation.as_deref()
    }
}

pub(super) fn observability_options(target: Option<&str>) -> RoutineInvocationOptions {
    RoutineInvocationOptions {
        target: target.map(str::to_owned),
        interruption: None,
        continuation: None,
    }
}

pub(crate) fn options(
    invocation: &ParsedInvocation,
) -> Result<RoutineInvocationOptions, PublicFailure> {
    if invocation.command != SuccessorCommand::Check(CheckProfile::Routine)
        || invocation.effect != EffectClass::WorkspaceWrite
    {
        return Err(PublicFailure::InvalidInvocation);
    }
    let mut target = None;
    let mut interruption = None;
    let mut continuation = None;
    for argument in &invocation.arguments {
        match (&argument.name, &argument.value) {
            (OptionName::Target, ParsedValue::RepositoryTarget(path)) if target.is_none() => {
                target = Some(path.as_str().to_owned());
            }
            (OptionName::InterruptAfter, ParsedValue::Identifier(value))
                if interruption.is_none() && value == "reservation" =>
            {
                interruption = Some(ReservationInterruption::AfterReservation);
            }
            (OptionName::Continuation, ParsedValue::Identifier(value))
                if continuation.is_none()
                    && value.starts_with("routine-cont-")
                    && value.len() > "routine-cont-".len() =>
            {
                continuation = Some(value.to_owned());
            }
            _ => return Err(PublicFailure::InvalidInvocation),
        }
    }
    Ok(RoutineInvocationOptions {
        target,
        interruption,
        continuation,
    })
}

pub(crate) fn target_root(
    root: &Path,
    options: &RoutineInvocationOptions,
) -> Result<PathBuf, PublicFailure> {
    let canonical_root = fs::canonicalize(root).map_err(|_| PublicFailure::Context)?;
    if root != Path::new(".") && canonical_root != root {
        return Err(PublicFailure::Context);
    }
    let requested = options
        .target
        .as_deref()
        .map_or_else(|| canonical_root.clone(), |path| canonical_root.join(path));
    let metadata = fs::symlink_metadata(&requested).map_err(|_| PublicFailure::Context)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(PublicFailure::Context);
    }
    let target = fs::canonicalize(&requested).map_err(|_| PublicFailure::Context)?;
    if !target.starts_with(&canonical_root) || target.to_str().is_none() {
        return Err(PublicFailure::Context);
    }
    Ok(target)
}

pub(crate) fn discovery_context(target: &Path) -> Result<LiveContext, PublicFailure> {
    LiveContext::build(
        BuildRequest::new(target)
            .expect_worktree_root(target)
            .with_effect(EffectClass::Read)
            .select_input(target.join(MANIFEST_PATH)),
    )
    .map_err(|_| PublicFailure::Context)
}

pub(crate) fn execution_context(
    target: &Path,
    manifest: &LoadedManifest,
) -> Result<LiveContext, PublicFailure> {
    let mut request = BuildRequest::new(target)
        .expect_worktree_root(target)
        .with_effect(EffectClass::Read)
        .bind_non_secret_configuration(SOURCE_CONFIG_KEY, manifest.source_id());
    for path in manifest.selected_paths() {
        request = request.select_input(target.join(path));
    }
    for tool in manifest.tool_names() {
        request = if tool == manifest::ROUTINE_RUNNER {
            request.probe_current_executable(manifest::ROUTINE_RUNNER)
        } else {
            request.probe_tool(tool)
        };
    }
    LiveContext::build(request).map_err(|_| PublicFailure::Context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::ADOPTED_HANDOFF_DIGEST_CONFIG_KEY;
    use std::process::Command;

    #[test]
    fn routine_contexts_ignore_removed_legacy_authority_inputs() {
        let root = minimal_repository("current-routine-context");
        fs::create_dir_all(root.join("config")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("config/routine-public.json"),
            include_bytes!("../../../../../../templates/config/routine-public.json"),
        )
        .unwrap();
        fs::write(
            root.join("config/routines.json"),
            include_bytes!("../../../../../../templates/config/routines.json"),
        )
        .unwrap();
        fs::write(root.join("src/lib.rs"), b"pub fn value() -> u8 { 1 }\n").unwrap();
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-qm", "install current routine"]);

        let discovery = match discovery_context(&root) {
            Ok(context) => context,
            Err(_) => panic!("current routine discovery context must build"),
        };
        assert!(discovery.configuration().public_values.is_empty());
        assert_eq!(
            discovery
                .selected_inputs()
                .iter()
                .map(|input| input.relative_path.as_str())
                .collect::<Vec<_>>(),
            [MANIFEST_PATH]
        );

        let manifest = manifest::load(&discovery, &root).unwrap();
        let execution = match execution_context(&root, &manifest) {
            Ok(context) => context,
            Err(_) => panic!("current routine execution context must build"),
        };
        assert_eq!(execution.configuration().public_values.len(), 1);
        assert_eq!(
            execution
                .configuration()
                .public_values
                .get(SOURCE_CONFIG_KEY),
            Some(&manifest.source_id())
        );
        assert!(
            !execution
                .configuration()
                .public_values
                .contains_key(ADOPTED_HANDOFF_DIGEST_CONFIG_KEY)
        );
        assert_eq!(
            execution
                .selected_inputs()
                .iter()
                .map(|input| input.relative_path.as_str())
                .collect::<Vec<_>>(),
            manifest
                .selected_paths()
                .iter()
                .map(|path| path.to_str().unwrap())
                .collect::<Vec<_>>()
        );
        assert!(
            execution
                .capabilities()
                .tool(manifest::ROUTINE_RUNNER)
                .is_some()
        );
        assert!(!root.join("docs").exists());
        assert!(!root.join("migration").exists());
        assert!(!root.join("validation_artifacts").exists());
        fs::remove_dir_all(root).unwrap();
    }

    fn minimal_repository(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-current-routine-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.email", "current@example.invalid"]);
        git(&root, &["config", "user.name", "Current Routine"]);
        root
    }

    fn git(root: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .args(arguments)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .current_dir(root)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {arguments:?}: {output:?}");
    }
}
