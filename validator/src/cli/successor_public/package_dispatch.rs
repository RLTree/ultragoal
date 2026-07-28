use super::*;

pub(super) fn execute(
    root: &Path,
    invocation: &ParsedInvocation,
    operation: operation_binding::PublicOperation,
) -> Option<RuntimeOutcome> {
    match operation {
        operation_binding::PublicOperation::PackageBuild => Some(with_package_contexts(
            root,
            |source_context, output_context| {
                package_build::execute(source_context, output_context, invocation)
            },
        )),
        operation_binding::PublicOperation::PackageVerify => {
            Some(with_read_context(root, |context| {
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
        _ => None,
    }
}

fn with_read_context(
    root: &Path,
    execute: impl FnOnce(&LiveContext) -> RuntimeOutcome,
) -> RuntimeOutcome {
    match read_context(root) {
        Ok(context) => execute(&context),
        Err(()) => context_unavailable(),
    }
}

fn with_package_contexts(
    root: &Path,
    execute: impl FnOnce(&LiveContext, &LiveContext) -> RuntimeOutcome,
) -> RuntimeOutcome {
    match (read_context(root), workspace_context(root)) {
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
