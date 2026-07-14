use super::contract::{PythonSourceLawAdapterError, PythonSourceLawResponse};

const MAX_DIAGNOSTIC_BYTES: usize = 8 * 1024;

pub(super) fn parse_pass(
    stdout: &[u8],
    stderr: &[u8],
) -> Result<PythonSourceLawResponse, PythonSourceLawAdapterError> {
    let text =
        std::str::from_utf8(stdout).map_err(|_| PythonSourceLawAdapterError::OutputInvalid)?;
    if !stderr.is_empty()
        || !text.starts_with("python source laws pass: ")
        || !text.ends_with(" typed sources\n")
    {
        return Err(PythonSourceLawAdapterError::OutputInvalid);
    }
    let count = text
        .trim_end_matches(" typed sources\n")
        .trim_start_matches("python source laws pass: ");
    count
        .parse::<usize>()
        .map_err(|_| PythonSourceLawAdapterError::OutputInvalid)?;
    Ok(PythonSourceLawResponse::new(Vec::new()))
}

pub(super) fn parse_findings(
    stdout: &[u8],
    stderr: &[u8],
) -> Result<PythonSourceLawResponse, PythonSourceLawAdapterError> {
    if !stdout.is_empty() {
        return Err(PythonSourceLawAdapterError::OutputInvalid);
    }
    let text =
        std::str::from_utf8(stderr).map_err(|_| PythonSourceLawAdapterError::OutputInvalid)?;
    let mut findings = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.len() > MAX_DIAGNOSTIC_BYTES || !valid_diagnostic(line) {
            return Err(PythonSourceLawAdapterError::OutputInvalid);
        }
        findings.push(line.to_owned());
    }
    if findings.is_empty() {
        return Err(PythonSourceLawAdapterError::OutputInvalid);
    }
    findings.sort();
    findings.dedup();
    Ok(PythonSourceLawResponse::new(findings))
}

fn valid_diagnostic(line: &str) -> bool {
    let mut fields = line.splitn(4, ':');
    if fields.next() != Some("python_source_law") {
        return false;
    }
    let Some(code) = fields.next() else {
        return false;
    };
    let Some(path) = fields.next() else {
        return false;
    };
    let Some(number) = fields.next() else {
        return false;
    };
    !code.is_empty()
        && code
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
        && !path.is_empty()
        && !path.starts_with('/')
        && !path.split('/').any(|part| matches!(part, "" | "." | ".."))
        && number.parse::<u64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::valid_diagnostic;

    #[test]
    fn rejects_ambiguous_or_unconfined_diagnostics() {
        assert!(valid_diagnostic(
            "python_source_law:function_return_type_missing:scripts/check.py:12"
        ));
        assert!(!valid_diagnostic(
            "python_source_law:bad-code:scripts/check.py:12"
        ));
        assert!(!valid_diagnostic("python_source_law:bad:/absolute.py:12"));
        assert!(!valid_diagnostic(
            "python_source_law:bad:scripts/../escape.py:12"
        ));
    }
}
