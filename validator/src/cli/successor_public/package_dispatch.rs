use super::*;

pub(super) fn execute(
    root: &Path,
    invocation: &ParsedInvocation,
    operation: operation_binding::PublicOperation,
) -> Option<RuntimeOutcome> {
    match operation {
        operation_binding::PublicOperation::PackageBuild => Some(with_context(root, |context| {
            package_build::execute(context, invocation)
        })),
        operation_binding::PublicOperation::PackageInventory => {
            Some(with_context(root, |context| {
                package_inventory::execute(context, invocation)
            }))
        }
        operation_binding::PublicOperation::PackageInstallTest => {
            Some(package_install_test::execute(root, invocation))
        }
        _ => None,
    }
}

fn with_context(
    root: &Path,
    execute: impl FnOnce(&LiveContext) -> RuntimeOutcome,
) -> RuntimeOutcome {
    match workspace_context(root) {
        Ok(context) => execute(&context),
        Err(()) => context_unavailable(),
    }
}
