#[cfg(test)]
fn totals(results: &[EvaluationTaskResult]) -> (u64, u64) {
    results.iter().fold((0, 0), |(earned, possible), result| {
        (
            earned.saturating_add(result.score_earned),
            possible.saturating_add(result.score_possible),
        )
    })
}

#[cfg(test)]
fn run_digest(
    spec: &EvaluationSpec,
    execution_session_id: &str,
    results: &[EvaluationTaskResult],
) -> String {
    run_digest_fields(
        &spec.spec_sha256,
        &spec.live_context_id,
        &spec.candidate_id,
        execution_session_id,
        results,
    )
}

#[cfg(test)]
fn run_digest_fields(
    spec_sha256: &str,
    live_context_id: &str,
    candidate_id: &str,
    execution_session_id: &str,
    results: &[EvaluationTaskResult],
) -> String {
    let rows = results
        .iter()
        .map(|result| {
            let controls = result
                .passed_perturbations
                .iter()
                .copied()
                .map(PerturbationControl::label)
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{}|{}|{}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                result.task_id,
                result.fixture_id,
                result.representative,
                result.outcome,
                result.causal_code,
                result.score_earned,
                result.score_possible,
                result.work_units,
                result.artifact_digest_sha256,
                result.producer_id,
                result.observer_id,
                result.scorer_id,
                result.independent_grader_id,
                result.independent_score_earned,
                result.independent_score_possible,
                controls,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    digest(
        format!("{spec_sha256}|{live_context_id}|{candidate_id}|{execution_session_id}|{rows}")
            .as_bytes(),
    )
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PATH_BYTES
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}
