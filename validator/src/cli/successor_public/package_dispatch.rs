use super::*;

pub(super) fn execute(
    root: &Path,
    invocation: &ParsedInvocation,
    operation: operation_binding::PublicOperation,
    home: Option<&Path>,
) -> Option<RuntimeOutcome> {
    match operation {
        operation_binding::PublicOperation::PackageBuild => Some(with_package_contexts(
            root,
            |source_context, output_context| {
                package_build::execute(source_context, output_context, invocation)
            },
        )),
        operation_binding::PublicOperation::PackageVerify => {
            Some(with_compatibility_read_context(root, |context| {
                package_verify::execute(context, invocation)
            }))
        }
        operation_binding::PublicOperation::PackageInventory => Some(with_package_contexts(
            root,
            |source_context, output_context| {
                package_inventory::execute(source_context, output_context, invocation)
            },
        )),
        operation_binding::PublicOperation::PackageInstallTest => Some(with_package_contexts(
            root,
            |source_context, output_context| {
                package_install_test::execute(source_context, output_context, invocation)
            },
        )),
        operation_binding::PublicOperation::PackageInstallPlan => {
            Some(with_lifecycle_contexts(root, |source, observation| {
                package_personal_install::plan(source, observation, invocation, home)
            }))
        }
        operation_binding::PublicOperation::PackageInstallVerify => {
            Some(with_lifecycle_contexts(root, |source, observation| {
                package_personal_install::verify(source, observation, invocation, home)
            }))
        }
        _ => None,
    }
}

fn with_lifecycle_contexts(
    root: &Path,
    execute: impl FnOnce(&LiveContext, &LiveContext) -> RuntimeOutcome,
) -> RuntimeOutcome {
    match (
        compatibility_read_context(root),
        compatibility_capabilities_context(root),
    ) {
        (Ok(source), Ok(observation)) => execute(&source, &observation),
        (Err(()), _) | (_, Err(())) => context_unavailable(),
    }
}

fn with_compatibility_read_context(
    root: &Path,
    execute: impl FnOnce(&LiveContext) -> RuntimeOutcome,
) -> RuntimeOutcome {
    match compatibility_read_context(root) {
        Ok(context) => execute(&context),
        Err(()) => context_unavailable(),
    }
}

fn with_package_contexts(
    root: &Path,
    execute: impl FnOnce(&LiveContext, &LiveContext) -> RuntimeOutcome,
) -> RuntimeOutcome {
    match (
        compatibility_read_context(root),
        compatibility_workspace_context(root),
    ) {
        (Ok(source_context), Ok(output_context)) => execute(&source_context, &output_context),
        (Err(()), _) | (_, Err(())) => context_unavailable(),
    }
}

pub(super) fn package_archive_input_allowed(path: &str) -> bool {
    let Some(name) = path.strip_prefix("target/ultragoal/") else {
        return false;
    };
    let Some(stem) = name.strip_suffix(".hugpkg") else {
        return false;
    };
    !stem.is_empty()
        && !name.contains('/')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::super::repository_fixture::Repository;
    use super::*;

    #[test]
    fn package_source_and_output_contexts_retain_compatibility_binding() {
        let repository = Repository::new("package-compatibility-context");
        let mut observed = false;

        let _ = with_package_contexts(&repository.root, |source, output| {
            observed = true;
            for context in [source, output] {
                assert_eq!(
                    context
                        .configuration()
                        .public_values
                        .get(ADOPTED_HANDOFF_DIGEST_CONFIG_KEY),
                    Some(&ADOPTED_HANDOFF_MANIFEST_SHA256.to_owned())
                );
            }
            context_unavailable()
        });

        assert!(observed);
    }
}
