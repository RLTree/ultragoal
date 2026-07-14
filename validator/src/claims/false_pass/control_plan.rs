use super::*;

impl SemanticControlPlan {
    pub(crate) fn issue(
        control: &NamedControlDefinition,
        context_id: &str,
        candidate_id: &str,
        nonce: &str,
    ) -> Result<Self, String> {
        if !digest(context_id) || !digest(candidate_id) || !digest(nonce) {
            return Err("claims-control-model-binding-not-digest".to_owned());
        }
        let model_implementation_digest = digest_value(MODEL_IMPLEMENTATION_VERSION);
        let model_spec_digest = digest_bytes(
            &serde_json::to_vec(&(
                &control.registry_digest,
                &control.claim_id,
                &control.control_id,
                control.control_ordinal,
                &control.definition_digest,
                &control.expected_failure_contract,
                &model_implementation_digest,
            ))
            .map_err(|_| "claims-control-model-spec-encode-failed".to_owned())?,
        );
        let negative_stimulus_digest = digest_bytes(
            &serde_json::to_vec(&(
                &control.claim_id,
                &control.control_id,
                control.control_ordinal,
                &control.definition_digest,
                &model_spec_digest,
                context_id,
                candidate_id,
                nonce,
            ))
            .map_err(|_| "claims-control-model-stimulus-encode-failed".to_owned())?,
        );
        let modeled_result_digest = digest_bytes(
            &serde_json::to_vec(&(
                "semantic-model-not-executed",
                &control.expected_failure_contract,
                &negative_stimulus_digest,
                &model_spec_digest,
            ))
            .map_err(|_| "claims-control-modeled-result-encode-failed".to_owned())?,
        );
        Ok(Self {
            model_spec_digest,
            negative_stimulus_digest,
            model_implementation_digest,
            modeled_result_digest,
        })
    }
}

pub(crate) fn candidate_id(context: &LiveContext) -> Result<String, String> {
    let bytes = serde_json::to_vec(context.candidate())
        .map_err(|_| "claims-control-candidate-encode-failed".to_owned())?;
    Ok(digest_bytes(&bytes))
}

pub(crate) fn validate_scratch_root(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("claims-control-scratch-root-invalid".to_owned());
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| "claims-control-scratch-root-unavailable".to_owned())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("claims-control-scratch-root-unsafe".to_owned());
    }
    #[cfg(unix)]
    if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
        return Err("claims-control-scratch-root-unsafe".to_owned());
    }
    path.canonicalize()
        .map_err(|_| "claims-control-scratch-root-unavailable".to_owned())
}

pub(crate) fn now_unix_ms() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "claims-control-clock-before-epoch".to_owned())
        .map(|duration| duration.as_millis() as u64)
}

pub(crate) fn short_digest(value: &str) -> &str {
    value
        .strip_prefix("sha256:")
        .and_then(|hex| hex.get(..16))
        .unwrap_or("invalid-digest")
}

pub(crate) fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

pub(crate) fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn digest_value(value: &str) -> String {
    digest_bytes(value.as_bytes())
}
