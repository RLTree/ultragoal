impl ResearchAudit {
    fn with(
        mut findings: Vec<ResearchFinding>,
        eligible_proposals: Vec<LawChangeProposal>,
    ) -> Self {
        findings.sort_by(|left, right| {
            (&left.code, &left.source_id, &left.proposal_id).cmp(&(
                &right.code,
                &right.source_id,
                &right.proposal_id,
            ))
        });
        Self {
            eligible_proposals,
            findings,
            authority_effect: NoAuthorityEffect::None,
        }
    }
}

fn invalid_source() -> EvaluationError {
    EvaluationError::new("evaluation-research-source-invalid")
}

fn invalid_proposal() -> EvaluationError {
    EvaluationError::new("evaluation-research-proposal-invalid")
}

fn valid_identifier(value: &str) -> bool {
    super::valid_identifier(value)
}

fn valid_identifier_set(values: &BTreeSet<String>, max: usize) -> bool {
    !values.is_empty() && values.len() <= max && values.iter().all(|value| valid_identifier(value))
}

fn valid_text(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && !value.chars().any(char::is_control)
        && value.trim() == value
}

fn valid_text_list(values: &[String]) -> bool {
    !values.is_empty()
        && values.len() <= MAX_CLASSIFICATION_ROWS
        && values.iter().all(|value| valid_text(value, MAX_TEXT_BYTES))
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn valid_nonempty_rows<T>(values: &[T]) -> bool {
    !values.is_empty() && values.len() <= MAX_CLASSIFICATION_ROWS
}

fn valid_https_url(value: &str) -> bool {
    if value.len() > MAX_URL_BYTES
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        || value.contains('#')
    {
        return false;
    }
    let Some(authority_and_path) = value.strip_prefix("https://") else {
        return false;
    };
    let authority = authority_and_path.split('/').next().unwrap_or_default();
    !authority.is_empty() && authority.contains('.') && !authority.contains('@')
}

fn valid_https_locator(value: &str) -> bool {
    let base = value.split('#').next().unwrap_or_default();
    valid_https_url(base)
        && value
            .strip_prefix(base)
            .is_some_and(|suffix| suffix.is_empty() || valid_fragment(suffix))
}

fn valid_fragment(value: &str) -> bool {
    value
        .strip_prefix('#')
        .is_some_and(|fragment| !fragment.is_empty() && valid_identifier(fragment))
}

fn valid_source_locator(locator: &str, source_url: &str) -> bool {
    locator == source_url || locator.strip_prefix(source_url).is_some_and(valid_fragment)
}

fn freshness_window_seconds(_facts: &[VerifiedSourceFact]) -> u64 {
    MUTABLE_CLAIM_FRESHNESS_SECONDS
}

fn parsed_checked_date(value: &str) -> Option<(u16, u8, u8)> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
    {
        return None;
    }
    let year = value[0..4].parse::<u16>().unwrap_or(0);
    let month = value[5..7].parse::<u8>().unwrap_or(0);
    let day = value[8..10].parse::<u8>().unwrap_or(0);
    if year < 2000 || !(1..=12).contains(&month) {
        return None;
    }
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month)
        .contains(&day)
        .then_some((year, month, day))
}

fn checked_date_matches_epoch(value: &str, epoch_seconds: u64) -> bool {
    let Some((year, month, day)) = parsed_checked_date(value) else {
        return false;
    };
    let days_before_year = (1970..year)
        .map(|candidate| {
            if candidate.is_multiple_of(4)
                && (!candidate.is_multiple_of(100) || candidate.is_multiple_of(400))
            {
                366_u64
            } else {
                365_u64
            }
        })
        .sum::<u64>();
    let days_before_month = (1..month)
        .map(|candidate| match candidate {
            2 if year.is_multiple_of(4)
                && (!year.is_multiple_of(100) || year.is_multiple_of(400)) =>
            {
                29_u64
            }
            2 => 28_u64,
            4 | 6 | 9 | 11 => 30_u64,
            _ => 31_u64,
        })
        .sum::<u64>();
    epoch_seconds / SECONDS_PER_DAY == days_before_year + days_before_month + u64::from(day - 1)
}

fn valid_sourced_row(
    id: &str,
    statement: &str,
    source_id: &str,
    source_locator: &str,
    mapped_law_ids: &BTreeSet<String>,
) -> bool {
    valid_identifier(id)
        && valid_text(statement, MAX_TEXT_BYTES)
        && valid_identifier(source_id)
        && valid_https_locator(source_locator)
        && valid_identifier_set(mapped_law_ids, MAX_CLASSIFICATION_ROWS)
}

fn valid_record_sourced_row<'a>(
    id: &'a str,
    statement: &'a str,
    source_id: &str,
    source_locator: &str,
    mapped_law_ids: &BTreeSet<String>,
    record: &ResearchSourceRecord,
    row_ids: &mut BTreeSet<&'a str>,
    statements: &mut BTreeSet<&'a str>,
) -> bool {
    source_id == record.source_id
        && valid_source_locator(source_locator, &record.url)
        && law_subset(mapped_law_ids, &record.mapped_law_ids)
        && row_ids.insert(id)
        && statements.insert(statement)
}

fn law_subset(values: &BTreeSet<String>, supported: &BTreeSet<String>) -> bool {
    !values.is_empty() && values.is_subset(supported)
}

fn valid_reference(value: &str) -> Option<&str> {
    valid_identifier(value).then_some(value)
}

fn finding(code: &str, source_id: Option<&str>, proposal_id: Option<&str>) -> ResearchFinding {
    ResearchFinding {
        code: code.to_owned(),
        source_id: source_id.map(str::to_owned),
        proposal_id: proposal_id.map(str::to_owned),
        authority_effect: NoAuthorityEffect::None,
    }
}
