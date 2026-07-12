use super::{
    EntryMatch, EntrySource, EnumerationBudget, FitError, FitErrorId,
    MAX_ENUMERATED_ENTRIES_PER_OPERATION, MAX_ENUMERATED_ENTRIES_PER_SCAN,
    MAX_ENUMERATED_NAME_BYTES_PER_SCAN, enumerate_and_close, error,
};

enum Step<'a> {
    Name(&'a [u8]),
    Error,
    End,
}

struct ScriptedEntries<'a> {
    steps: std::slice::Iter<'a, Step<'a>>,
    closed: bool,
    close_fails: bool,
}

impl<'a> ScriptedEntries<'a> {
    fn new(steps: &'a [Step<'a>]) -> Self {
        Self {
            steps: steps.iter(),
            closed: false,
            close_fails: false,
        }
    }
}

impl EntrySource for ScriptedEntries<'_> {
    fn next(&mut self) -> Result<Option<&[u8]>, FitError> {
        match self.steps.next().unwrap_or(&Step::End) {
            Step::Name(name) => Ok(Some(name)),
            Step::Error => Err(error(FitErrorId::ReadFailed)),
            Step::End => Ok(None),
        }
    }

    fn close(&mut self) -> Result<(), FitError> {
        self.closed = true;
        if self.close_fails {
            Err(error(FitErrorId::ReadFailed))
        } else {
            Ok(())
        }
    }
}

fn run(steps: &[Step<'_>], close_fails: bool) -> (Result<EntryMatch, FitError>, bool) {
    let mut budget = EnumerationBudget::new();
    run_with_budget(steps, close_fails, &mut budget)
}

fn run_with_budget(
    steps: &[Step<'_>],
    close_fails: bool,
    budget: &mut EnumerationBudget,
) -> (Result<EntryMatch, FitError>, bool) {
    let mut entries = ScriptedEntries::new(steps);
    entries.close_fails = close_fails;
    let result = enumerate_and_close(&mut entries, "AGENTS.md", budget);
    (result, entries.closed)
}

fn names(count: usize, name: &'static [u8]) -> Vec<Step<'static>> {
    std::iter::repeat_with(|| Step::Name(name))
        .take(count)
        .chain(std::iter::once(Step::End))
        .collect()
}

fn error_after_names(count: usize) -> Vec<Step<'static>> {
    std::iter::repeat_with(|| Step::Name(b"other"))
        .take(count)
        .chain(std::iter::once(Step::Error))
        .collect()
}

#[test]
fn complete_scans_classify_only_after_normal_eof() {
    for (steps, expected) in [
        (
            &[Step::Name(b"AGENTS.md"), Step::End][..],
            EntryMatch::Exact,
        ),
        (
            &[Step::Name(b"Agents.md"), Step::End][..],
            EntryMatch::Alias,
        ),
        (
            &[Step::Name(b"AGENT.md"), Step::End][..],
            EntryMatch::Absent,
        ),
        (
            &[
                Step::Name(b"AGENTS.md"),
                Step::Name(b"Agents.md"),
                Step::End,
            ][..],
            EntryMatch::Alias,
        ),
        (
            &[
                Step::Name(b"Agents.md"),
                Step::Name(b"AGENTS.md"),
                Step::End,
            ][..],
            EntryMatch::Alias,
        ),
    ] {
        let (result, closed) = run(steps, false);
        assert_eq!(result.unwrap(), expected);
        assert!(closed);
    }
}

#[test]
fn enumeration_error_discards_every_partial_observation() {
    for steps in [
        &[Step::Name(b"AGENTS.md"), Step::Error][..],
        &[Step::Error][..],
        &[Step::Name(b"Agents.md"), Step::Error][..],
    ] {
        let (result, closed) = run(steps, false);
        assert_eq!(result.unwrap_err().id(), FitErrorId::ReadFailed);
        assert!(closed);
    }
}

#[test]
fn enumeration_error_is_charged_at_the_per_scan_boundary() {
    let (result, closed) = run(
        &error_after_names(MAX_ENUMERATED_ENTRIES_PER_SCAN - 1),
        false,
    );
    assert_eq!(result.unwrap_err().id(), FitErrorId::ReadFailed);
    assert!(closed);

    let (result, closed) = run(&error_after_names(MAX_ENUMERATED_ENTRIES_PER_SCAN), false);
    assert_eq!(result.unwrap_err().id(), FitErrorId::ResourceLimit);
    assert!(closed);
}

#[test]
fn enumeration_error_is_charged_at_the_cumulative_boundary() {
    let complete_scan = names(MAX_ENUMERATED_ENTRIES_PER_SCAN, b"other");
    let mut budget = EnumerationBudget::new();
    for _ in 0..MAX_ENUMERATED_ENTRIES_PER_OPERATION / MAX_ENUMERATED_ENTRIES_PER_SCAN - 1 {
        let (result, closed) = run_with_budget(&complete_scan, false, &mut budget);
        assert_eq!(result.unwrap(), EntryMatch::Absent);
        assert!(closed);
    }
    let final_partial = names(MAX_ENUMERATED_ENTRIES_PER_SCAN - 1, b"other");
    let (result, closed) = run_with_budget(&final_partial, false, &mut budget);
    assert_eq!(result.unwrap(), EntryMatch::Absent);
    assert!(closed);

    let (result, closed) = run_with_budget(&[Step::Error], false, &mut budget);
    assert_eq!(result.unwrap_err().id(), FitErrorId::ReadFailed);
    assert!(closed);

    let (result, closed) = run_with_budget(&[Step::Error], false, &mut budget);
    assert_eq!(result.unwrap_err().id(), FitErrorId::ResourceLimit);
    assert!(closed);
}

#[test]
fn enumeration_error_at_the_exact_name_byte_boundary_remains_content_free() {
    let exact = vec![b'a'; MAX_ENUMERATED_NAME_BYTES_PER_SCAN];
    let steps = [Step::Name(&exact), Step::Error];
    let (result, closed) = run(&steps, false);
    assert_eq!(result.unwrap_err().id(), FitErrorId::ReadFailed);
    assert!(closed);
}

#[test]
fn malformed_entry_and_close_failure_fail_closed() {
    let invalid = [Step::Name(b"\xff"), Step::End];
    let (result, closed) = run(&invalid, false);
    assert_eq!(result.unwrap_err().id(), FitErrorId::UnsafeObject);
    assert!(closed);

    let exact = [Step::Name(b"AGENTS.md"), Step::End];
    let (result, closed) = run(&exact, true);
    assert_eq!(result.unwrap_err().id(), FitErrorId::ReadFailed);
    assert!(closed);
}

#[test]
fn enumeration_budget_allows_the_exact_limit_and_rejects_one_over() {
    let exact = names(MAX_ENUMERATED_ENTRIES_PER_SCAN - 1, b"other");
    let mut exact = exact;
    exact.insert(
        MAX_ENUMERATED_ENTRIES_PER_SCAN - 1,
        Step::Name(b"AGENTS.md"),
    );
    let (result, closed) = run(&exact, false);
    assert_eq!(result.unwrap(), EntryMatch::Exact);
    assert!(closed);

    let over = names(MAX_ENUMERATED_ENTRIES_PER_SCAN + 1, b"other");
    let (result, closed) = run(&over, false);
    assert_eq!(result.unwrap_err().id(), FitErrorId::ResourceLimit);
    assert!(closed);
}

#[test]
fn budget_rejects_an_alias_after_the_scan_boundary_without_partial_success() {
    let mut steps = names(MAX_ENUMERATED_ENTRIES_PER_SCAN - 1, b"other");
    steps.insert(
        MAX_ENUMERATED_ENTRIES_PER_SCAN - 1,
        Step::Name(b"AGENTS.md"),
    );
    steps.insert(MAX_ENUMERATED_ENTRIES_PER_SCAN, Step::Name(b"Agents.md"));
    let (result, closed) = run(&steps, false);
    assert_eq!(result.unwrap_err().id(), FitErrorId::ResourceLimit);
    assert!(closed);
}

#[test]
fn cumulative_budget_rejects_repeated_complete_scans() {
    let scan = names(MAX_ENUMERATED_ENTRIES_PER_SCAN, b"other");
    let mut budget = EnumerationBudget::new();
    for _ in 0..MAX_ENUMERATED_ENTRIES_PER_OPERATION / MAX_ENUMERATED_ENTRIES_PER_SCAN {
        let (result, closed) = run_with_budget(&scan, false, &mut budget);
        assert_eq!(result.unwrap(), EntryMatch::Absent);
        assert!(closed);
    }
    let (result, closed) = run_with_budget(&scan, false, &mut budget);
    assert_eq!(result.unwrap_err().id(), FitErrorId::ResourceLimit);
    assert!(closed);
}

#[test]
fn churning_entry_stream_stops_without_retrying_or_returning_a_partial_match() {
    let mut steps = names(MAX_ENUMERATED_ENTRIES_PER_SCAN, b"churn");
    steps.insert(0, Step::Name(b"AGENTS.md"));
    let (result, closed) = run(&steps, false);
    assert_eq!(result.unwrap_err().id(), FitErrorId::ResourceLimit);
    assert!(closed);
}

#[test]
fn enumeration_budget_rejects_an_oversized_name_stream() {
    let name = vec![b'a'; MAX_ENUMERATED_NAME_BYTES_PER_SCAN + 1];
    let steps = [Step::Name(&name), Step::End];
    let (result, closed) = run(&steps, false);
    assert_eq!(result.unwrap_err().id(), FitErrorId::ResourceLimit);
    assert!(closed);
}
