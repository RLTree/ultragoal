use super::*;

pub(crate) fn normalized_output_scopes(
    mut scopes: Vec<RepoPath>,
) -> Result<Vec<RepoPath>, RoutineError> {
    scopes.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    let mut case_keys = BTreeSet::new();
    if scopes
        .iter()
        .any(|scope| !case_keys.insert(scope.case_key()))
    {
        return Err(adapter_error("adapter-output-scope-duplicated"));
    }
    Ok(scopes)
}

pub(crate) fn validate_result_scope(value: &str) -> Result<(), RoutineError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(adapter_error("adapter-result-scope-invalid"));
    }
    Ok(())
}

pub(crate) fn request_seal(request_id: &str, protocol_id: &str, issuance: u64) -> String {
    framed(&[
        REQUEST_SEAL_DOMAIN,
        request_id.as_bytes(),
        protocol_id.as_bytes(),
        &issuance.to_be_bytes(),
    ])
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn adapter_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}
