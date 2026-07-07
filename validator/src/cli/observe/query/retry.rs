use crate::cli::observe::types::ObserveCommand;
use std::thread;
use std::time::{Duration, Instant};

pub(super) const NO_MATCHING_ROWS: &str = "observability query returned no matching rows";

pub(crate) fn retry_until_reconciled<F, V>(
    command: &ObserveCommand,
    mut fetch: F,
    mut validate: V,
) -> Result<String, String>
where
    F: FnMut() -> Result<String, String>,
    V: FnMut(&str) -> Option<String>,
{
    let deadline = Instant::now() + Duration::from_millis(command.timeout_ms.max(1));
    let mut last_body_failure = None;
    let mut last_error = NO_MATCHING_ROWS.to_string();
    loop {
        match fetch() {
            Ok(body) => match validate(&body) {
                None => return Ok(body),
                Some(failure) => {
                    last_body_failure = Some((body, failure));
                }
            },
            Err(err) => {
                last_error = err;
            }
        }
        if Instant::now() >= deadline {
            return match last_body_failure {
                Some((body, failure)) if failure != NO_MATCHING_ROWS => Ok(body),
                Some((_, failure)) if last_error == NO_MATCHING_ROWS => Err(failure),
                Some(_) | None => Err(last_error),
            };
        }
        thread::sleep(Duration::from_millis(250));
    }
}

#[cfg(test)]
pub(crate) fn retry_until_match_for_test(
    command: &ObserveCommand,
    mut outputs: Vec<Result<String, String>>,
) -> Result<String, String> {
    outputs.reverse();
    retry_until_match(command, || {
        outputs
            .pop()
            .unwrap_or_else(|| Err("test outputs exhausted".to_string()))
    })
}

#[cfg(test)]
fn retry_until_match<F>(command: &ObserveCommand, fetch: F) -> Result<String, String>
where
    F: FnMut() -> Result<String, String>,
{
    retry_until_reconciled(command, fetch, |body| {
        if super::has_matches(body, command.operation) {
            None
        } else {
            Some(NO_MATCHING_ROWS.to_string())
        }
    })
}
