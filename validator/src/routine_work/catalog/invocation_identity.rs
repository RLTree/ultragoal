use super::*;

impl<'a> From<&'a BoundCatalogInvocation> for InvocationIdentity<'a> {
    fn from(value: &'a BoundCatalogInvocation) -> Self {
        Self {
            catalog_id: &value.catalog_id,
            graph_id: &value.graph_id,
            candidate_id: &value.candidate_id,
            plan_id: &value.plan_id,
            definition_id: &value.definition_id,
            definition_sha256: &value.definition_sha256,
            node_id: &value.node_id,
            behavior_id: &value.behavior_id,
            depends_on: &value.depends_on,
            selected_tool: &value.selected_tool,
            selected_tool_identity_sha256: &value.selected_tool_identity_sha256,
            used_fallback: value.used_fallback,
            input_id: &value.input_id,
            program_path_hex: &value.program_path_hex,
            program_sha256: &value.program_sha256,
            program_byte_length: value.program_byte_length,
            program_unix_mode: value.program_unix_mode,
            arguments: &value.arguments,
            environment: &value.environment,
            read_sources: &value.read_sources,
            timeout_ms: value.timeout_ms,
            output_budget_bytes: value.output_budget_bytes,
            output_scopes: &value.output_scopes,
            runner_policy: value.runner_policy,
            read_policy: value.read_policy,
        }
    }
}

#[derive(Serialize)]
pub(crate) struct InvocationSetIdentity<'a> {
    pub(crate) catalog_id: &'a str,
    pub(crate) graph_id: &'a str,
    pub(crate) candidate_id: &'a str,
    pub(crate) plan_id: &'a str,
    pub(crate) invocation_ids: Vec<&'a str>,
}

pub(crate) fn identifier(value: String) -> CatalogResult<String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(error("catalog-semantic-identifier-invalid"));
    }
    Ok(value)
}

pub(crate) fn identifier_set(
    values: impl IntoIterator<Item = String>,
    duplicate_error: &'static str,
) -> CatalogResult<BTreeSet<String>> {
    let mut result = BTreeSet::new();
    let mut case_values = BTreeSet::new();
    for value in values {
        let value = identifier(value)?;
        if !case_values.insert(value.to_ascii_lowercase()) {
            return Err(error(duplicate_error));
        }
        if !result.insert(value) {
            return Err(error(duplicate_error));
        }
    }
    Ok(result)
}

pub(crate) fn required_sha256(value: String, error_code: &'static str) -> CatalogResult<String> {
    if value.len() != 71
        || !value.starts_with("sha256:")
        || !value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(error(error_code));
    }
    Ok(value)
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn digest_json<T: Serialize + ?Sized>(value: &T) -> CatalogResult<String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| error("catalog-identity-serialization-failed"))
}

pub(crate) fn hex_path(path: &Path) -> CatalogResult<String> {
    let bytes = path
        .to_str()
        .ok_or_else(|| error("catalog-runner-path-invalid"))?
        .as_bytes();
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(crate) fn error(code: &'static str) -> RoutineCatalogError {
    RoutineCatalogError::new(code)
}
