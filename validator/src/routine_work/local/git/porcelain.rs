use std::collections::BTreeSet;

use crate::routine_work::{ChangeKind, RepoPath, RoutineError, RoutineErrorId};

const STATUS_LIMIT: usize = 16 * 1024 * 1024;
const STATUS_ROW_LIMIT: usize = 100_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StatusRow {
    path: RepoPath,
    previous_path: Option<RepoPath>,
    kind: ChangeKind,
}

impl StatusRow {
    pub(crate) fn path(&self) -> &RepoPath {
        &self.path
    }

    pub(crate) fn previous_path(&self) -> Option<&RepoPath> {
        self.previous_path.as_ref()
    }

    pub(crate) fn kind(&self) -> ChangeKind {
        self.kind
    }

    pub(crate) fn needs_content(&self) -> bool {
        !matches!(self.kind, ChangeKind::Deleted | ChangeKind::Conflict)
    }
}

pub(crate) fn parse_status(bytes: &[u8]) -> Result<Vec<StatusRow>, RoutineError> {
    if bytes.len() > STATUS_LIMIT {
        return Err(limit_error("git-status-output-limit-exceeded"));
    }
    let records = bytes.split(|byte| *byte == 0).collect::<Vec<_>>();
    if records.len() > STATUS_ROW_LIMIT.saturating_mul(2).saturating_add(1) {
        return Err(limit_error("git-status-row-limit-exceeded"));
    }
    let mut rows = Vec::new();
    let mut index = 0;
    while index < records.len() {
        let record = records[index];
        index += 1;
        if record.is_empty() {
            continue;
        }
        let row = match record.first().copied() {
            Some(b'1') => parse_ordinary(record)?,
            Some(b'2') => {
                let prior = records
                    .get(index)
                    .copied()
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| capture_error("rename-origin-missing"))?;
                index += 1;
                parse_rename(record, prior)?
            }
            Some(b'u') => parse_unmerged(record)?,
            Some(b'?') => StatusRow {
                path: parse_path(
                    record
                        .get(2..)
                        .ok_or_else(|| capture_error("untracked-row-short"))?,
                )?,
                previous_path: None,
                kind: ChangeKind::Untracked,
            },
            Some(b'!') => continue,
            _ => return Err(capture_error("git-status-record-unknown")),
        };
        rows.push(row);
        if rows.len() > STATUS_ROW_LIMIT {
            return Err(limit_error("git-status-row-limit-exceeded"));
        }
    }
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    validate_unique_paths(&rows)?;
    Ok(rows)
}

fn parse_ordinary(record: &[u8]) -> Result<StatusRow, RoutineError> {
    let text = std::str::from_utf8(record).map_err(|_| capture_error("git-status-non-utf8"))?;
    let fields = text.splitn(9, ' ').collect::<Vec<_>>();
    if fields.len() != 9
        || fields[0] != "1"
        || !valid_xy(fields[1])
        || fields[1].bytes().any(|byte| matches!(byte, b'R' | b'C'))
    {
        return Err(capture_error("ordinary-row-invalid"));
    }
    validate_tracked_metadata(&fields[2..8])?;
    Ok(StatusRow {
        path: RepoPath::parse(fields[8].to_owned())?,
        previous_path: None,
        kind: kind_from_xy(fields[1]),
    })
}

fn parse_rename(record: &[u8], prior: &[u8]) -> Result<StatusRow, RoutineError> {
    let text = std::str::from_utf8(record).map_err(|_| capture_error("git-status-non-utf8"))?;
    let fields = text.splitn(10, ' ').collect::<Vec<_>>();
    if fields.len() != 10
        || fields[0] != "2"
        || !valid_xy(fields[1])
        || !fields[1].bytes().any(|byte| matches!(byte, b'R' | b'C'))
        || !valid_score(fields[8])
    {
        return Err(capture_error("rename-row-invalid"));
    }
    validate_tracked_metadata(&fields[2..8])?;
    Ok(StatusRow {
        path: RepoPath::parse(fields[9].to_owned())?,
        previous_path: Some(parse_path(prior)?),
        kind: ChangeKind::Renamed,
    })
}

fn parse_unmerged(record: &[u8]) -> Result<StatusRow, RoutineError> {
    let text = std::str::from_utf8(record).map_err(|_| capture_error("git-status-non-utf8"))?;
    let fields = text.splitn(11, ' ').collect::<Vec<_>>();
    if fields.len() != 11 || fields[0] != "u" || !valid_xy(fields[1]) || !fields[1].contains('U') {
        return Err(capture_error("unmerged-row-invalid"));
    }
    if !valid_submodule(fields[2])
        || fields[3..7].iter().any(|field| !valid_mode(field))
        || fields[7..10].iter().any(|field| !valid_oid(field))
    {
        return Err(capture_error("unmerged-metadata-invalid"));
    }
    reject_submodule(fields[2])?;
    Ok(StatusRow {
        path: RepoPath::parse(fields[10].to_owned())?,
        previous_path: None,
        kind: ChangeKind::Conflict,
    })
}

fn validate_tracked_metadata(fields: &[&str]) -> Result<(), RoutineError> {
    if fields.len() != 6
        || !valid_submodule(fields[0])
        || fields[1..4].iter().any(|field| !valid_mode(field))
        || fields[4..6].iter().any(|field| !valid_oid(field))
    {
        return Err(capture_error("tracked-metadata-invalid"));
    }
    reject_submodule(fields[0])
}

fn reject_submodule(value: &str) -> Result<(), RoutineError> {
    if value.starts_with('S') {
        Err(RoutineError::new(
            RoutineErrorId::UnsupportedEntry,
            "dirty-submodule-not-supported",
            None,
        ))
    } else {
        Ok(())
    }
}

fn valid_xy(value: &str) -> bool {
    value.len() == 2
        && value
            .bytes()
            .all(|byte| matches!(byte, b'.' | b'M' | b'T' | b'A' | b'D' | b'R' | b'C' | b'U'))
}

fn valid_submodule(value: &str) -> bool {
    value.len() == 4
        && matches!(value.as_bytes()[0], b'N' | b'S')
        && value.as_bytes()[1..]
            .iter()
            .all(|byte| matches!(byte, b'.' | b'C' | b'M' | b'U'))
        && (value.starts_with('S') || value == "N...")
}

fn valid_mode(value: &str) -> bool {
    value.len() == 6 && value.bytes().all(|byte| matches!(byte, b'0'..=b'7'))
}

fn valid_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_score(value: &str) -> bool {
    matches!(value.as_bytes().first(), Some(b'R' | b'C'))
        && value.len() > 1
        && value[1..].bytes().all(|byte| byte.is_ascii_digit())
        && value[1..].parse::<u8>().is_ok_and(|score| score <= 100)
}

fn kind_from_xy(xy: &str) -> ChangeKind {
    if xy.bytes().any(|byte| byte == b'U') {
        ChangeKind::Conflict
    } else if xy.bytes().any(|byte| byte == b'D') {
        ChangeKind::Deleted
    } else if xy.bytes().any(|byte| byte == b'A') {
        ChangeKind::Added
    } else {
        ChangeKind::Modified
    }
}

fn validate_unique_paths(rows: &[StatusRow]) -> Result<(), RoutineError> {
    let mut exact = BTreeSet::new();
    let mut folded = BTreeSet::new();
    for row in rows {
        if !exact.insert(row.path.clone()) {
            return Err(invalid_snapshot("git-status-path-duplicated"));
        }
        if !folded.insert(row.path.case_key()) {
            return Err(invalid_snapshot("git-status-path-case-aliased"));
        }
    }
    Ok(())
}

fn parse_path(bytes: &[u8]) -> Result<RepoPath, RoutineError> {
    let text = std::str::from_utf8(bytes).map_err(|_| capture_error("git-status-non-utf8"))?;
    RepoPath::parse(text.to_owned())
}

fn capture_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CaptureFailed, cause, None)
}

fn limit_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CaptureLimit, cause, None)
}

fn invalid_snapshot(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidSnapshot, cause, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_unknown_and_case_aliased_rows_fail_closed() {
        assert_eq!(
            parse_status(b"x malformed\0").unwrap_err().id(),
            RoutineErrorId::CaptureFailed
        );
        let rows = b"? src/Foo.rs\0? src/foo.rs\0";
        assert_eq!(
            parse_status(rows).unwrap_err().id(),
            RoutineErrorId::InvalidSnapshot
        );
    }

    #[test]
    fn submodule_metadata_is_rejected_instead_of_treated_as_a_file() {
        let oid = "0".repeat(40);
        let row = format!("1 .M S.M. 160000 160000 160000 {oid} {oid} vendor/sub\0");
        assert_eq!(
            parse_status(row.as_bytes()).unwrap_err().id(),
            RoutineErrorId::UnsupportedEntry
        );
    }
}
