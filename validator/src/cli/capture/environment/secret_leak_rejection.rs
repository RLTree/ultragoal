use super::*;

pub(crate) fn reject_secret_in_public_fields(
    spec: &CommandSpec,
    context: &LiveContext,
    secrets: &[Vec<u8>],
) -> Result<(), String> {
    let public = std::iter::once(os_bytes(spec.program.as_os_str()))
        .chain(std::iter::once(os_bytes(spec.cwd.as_os_str())))
        .chain(spec.arguments.iter().filter_map(|input| match input {
            ArgumentInput::Public(item) => Some(os_bytes(&item.value)),
            ArgumentInput::Secret(_) => None,
        }))
        .chain(spec.environment.iter().flat_map(|input| match input {
            EnvironmentInput::Public(item) => [os_bytes(&item.name), os_bytes(&item.value)],
            EnvironmentInput::Secret(item) => [os_bytes(&item.name), item.source.as_bytes()],
        }))
        .chain(
            spec.artifacts
                .iter()
                .flat_map(|input| match input {
                    ArtifactExpectation::Public(item) => [
                        Some(os_bytes(item.path.as_os_str())),
                        item.expected_sha256.as_deref().map(str::as_bytes),
                    ],
                    ArtifactExpectation::Secret(_) => [None, None],
                })
                .flatten(),
        )
        .chain(
            context
                .configuration()
                .secret_sources
                .iter()
                .flat_map(|item| [item.name.as_bytes(), item.public_version.as_bytes()]),
        );
    if public
        .into_iter()
        .any(|field| secrets.iter().any(|secret| contains(field, secret)))
    {
        return Err("secret value appeared outside an explicit secret channel".to_owned());
    }
    Ok(())
}
