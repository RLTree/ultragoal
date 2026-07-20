use super::{
    CanonicalPath, ExpectedContent, FitCheck, FitError, FitErrorId, FitReader, Mutation,
    Ownership, OwnershipProvenance, digest, error,
};

pub(crate) const LOCAL_STATE_PATH: &str = ".gitignore";
pub(crate) const LOCAL_STATE_RULE: &str = "validation_artifacts/";
pub(crate) const DEFAULT_LOCAL_STATE_MODE: u32 = 0o644;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LocalStateDisposition {
    Missing,
    NeedsUpdate,
    AlreadyIgnored,
}

#[derive(Clone, Debug)]
pub(crate) struct LocalStatePlan {
    pub(crate) path: CanonicalPath,
    pub(crate) required_rule: String,
    pub(crate) disposition: LocalStateDisposition,
    pub(crate) expected: ExpectedContent,
    pub(crate) observed_mode: Option<u32>,
    pub(crate) desired_mode: u32,
    pub(crate) replacement: Vec<u8>,
    pub(crate) prior: Option<Vec<u8>>,
    pub(crate) mutation: Option<Mutation>,
}

impl LocalStatePlan {
    pub(crate) fn desired_sha256(&self) -> String {
        digest(&self.replacement)
    }

    pub(crate) fn check(&self) -> FitCheck {
        FitCheck {
            path: self.path.clone(),
            expected: self.expected.clone(),
            desired_sha256: self.desired_sha256(),
            provenance: OwnershipProvenance::UserDeclared,
            prior_proof_sha256: None,
        }
    }

    pub(crate) fn mutation_count(&self) -> usize {
        usize::from(self.mutation.is_some())
    }
}

pub(crate) fn inspect_local_state(
    reader: &mut impl FitReader,
    observed_mode: Option<u32>,
) -> Result<LocalStatePlan, FitError> {
    let path = CanonicalPath::parse(LOCAL_STATE_PATH)?;
    let prior = reader.read_file(&path, super::repository_contract::MAX_FILE_BYTES)?;
    if prior.is_none() != observed_mode.is_none() {
        return Err(error(FitErrorId::StaleBinding));
    }
    let disposition = match prior.as_deref() {
        None => LocalStateDisposition::Missing,
        Some(bytes) if ignores_validation_artifacts(bytes) => LocalStateDisposition::AlreadyIgnored,
        Some(_) => LocalStateDisposition::NeedsUpdate,
    };
    let replacement = match disposition {
        LocalStateDisposition::AlreadyIgnored => prior.clone().expect("present policy file"),
        LocalStateDisposition::Missing => format!("{LOCAL_STATE_RULE}\n").into_bytes(),
        LocalStateDisposition::NeedsUpdate => append_rule(prior.as_deref().expect("present file")),
    };
    let expected = prior
        .as_deref()
        .map(digest)
        .map(ExpectedContent::ExactDigest)
        .unwrap_or(ExpectedContent::Absent);
    let desired_mode = observed_mode.unwrap_or(DEFAULT_LOCAL_STATE_MODE);
    let mutation = (disposition != LocalStateDisposition::AlreadyIgnored).then(|| Mutation {
        path: path.clone(),
        expected: expected.clone(),
        replacement: replacement.clone(),
        prior: prior.clone(),
        ownership: Ownership::UserOwned,
        provenance: OwnershipProvenance::UserDeclared,
        prior_proof_sha256: None,
    });
    Ok(LocalStatePlan {
        path,
        required_rule: LOCAL_STATE_RULE.to_owned(),
        disposition,
        expected,
        observed_mode,
        desired_mode,
        replacement,
        prior,
        mutation,
    })
}

fn ignores_validation_artifacts(bytes: &[u8]) -> bool {
    let mut ignored = false;
    for line in bytes.split(|byte| *byte == b'\n') {
        let line = trim_ascii_space(line);
        if line.is_empty() || line.first() == Some(&b'#') {
            continue;
        }
        let (negated, pattern) = match line.first() {
            Some(b'!') => (true, &line[1..]),
            Some(_) => (false, line),
            None => continue,
        };
        let pattern = trim_ascii_space(pattern);
        let pattern = pattern.strip_prefix(b"/").unwrap_or(pattern);
        let pattern = pattern.strip_prefix(b"**/").unwrap_or(pattern);
        let pattern = pattern.strip_suffix(b"/").unwrap_or(pattern);
        let pattern = pattern.strip_suffix(b"/**").unwrap_or(pattern);
        if pattern == b"validation_artifacts" {
            ignored = !negated;
        }
    }
    ignored
}

fn append_rule(bytes: &[u8]) -> Vec<u8> {
    let mut replacement = Vec::with_capacity(bytes.len() + LOCAL_STATE_RULE.len() + 2);
    replacement.extend_from_slice(bytes);
    if bytes.ends_with(b"\r\n") {
        replacement.extend_from_slice(LOCAL_STATE_RULE.as_bytes());
        replacement.extend_from_slice(b"\r\n");
    } else if bytes.last().is_some_and(|byte| *byte == b'\n') {
        replacement.extend_from_slice(LOCAL_STATE_RULE.as_bytes());
        replacement.push(b'\n');
    } else {
        let newline: &[u8] = if bytes.windows(2).any(|pair| pair == b"\r\n") {
            b"\r\n"
        } else {
            b"\n"
        };
        replacement.extend_from_slice(newline);
        replacement.extend_from_slice(LOCAL_STATE_RULE.as_bytes());
    }
    replacement
}

fn trim_ascii_space(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !matches!(byte, b' ' | b'\t' | b'\r'))
        .unwrap_or(bytes.len());
    let end = bytes[..]
        .iter()
        .rposition(|byte| !matches!(byte, b' ' | b'\t' | b'\r'))
        .map_or(start, |index| index + 1);
    &bytes[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_rule_respects_comments_and_last_negation() {
        assert!(ignores_validation_artifacts(
            b"# keep this\n/validation_artifacts/\n"
        ));
        assert!(!ignores_validation_artifacts(
            b"validation_artifacts/\n!validation_artifacts/\n"
        ));
        assert!(ignores_validation_artifacts(b"**/validation_artifacts\n"));
        assert!(ignores_validation_artifacts(b"validation_artifacts/**\n"));
    }

    #[test]
    fn replacement_preserves_final_newline_shape_and_user_bytes() {
        assert_eq!(
            append_rule(b"# note\n!validation_artifacts/"),
            b"# note\n!validation_artifacts/\nvalidation_artifacts/"
        );
        assert_eq!(
            append_rule(b"# note\n"),
            b"# note\nvalidation_artifacts/\n"
        );
        assert_eq!(
            append_rule(b"# note\r\n"),
            b"# note\r\nvalidation_artifacts/\r\n"
        );
    }
}
