use super::*;

pub(crate) const STATUS_LIMIT: usize = 16 * 1024 * 1024;
pub(crate) const STATUS_ROW_LIMIT: usize = 100_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StatusRow {
    pub(crate) path: RepoPath,
    pub(crate) previous_path: Option<RepoPath>,
    pub(crate) kind: ChangeKind,
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

pub(crate) fn parse_ordinary(record: &[u8]) -> Result<StatusRow, RoutineError> {
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

pub(crate) fn parse_rename(record: &[u8], prior: &[u8]) -> Result<StatusRow, RoutineError> {
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

pub(crate) fn parse_unmerged(record: &[u8]) -> Result<StatusRow, RoutineError> {
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

pub(crate) fn validate_tracked_metadata(fields: &[&str]) -> Result<(), RoutineError> {
    if fields.len() != 6
        || !valid_submodule(fields[0])
        || fields[1..4].iter().any(|field| !valid_mode(field))
        || fields[4..6].iter().any(|field| !valid_oid(field))
    {
        return Err(capture_error("tracked-metadata-invalid"));
    }
    reject_submodule(fields[0])
}

pub(crate) fn reject_submodule(value: &str) -> Result<(), RoutineError> {
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

pub(crate) fn valid_xy(value: &str) -> bool {
    value.len() == 2
        && value
            .bytes()
            .all(|byte| matches!(byte, b'.' | b'M' | b'T' | b'A' | b'D' | b'R' | b'C' | b'U'))
}

pub(crate) fn valid_submodule(value: &str) -> bool {
    value.len() == 4
        && matches!(value.as_bytes()[0], b'N' | b'S')
        && value.as_bytes()[1..]
            .iter()
            .all(|byte| matches!(byte, b'.' | b'C' | b'M' | b'U'))
        && (value.starts_with('S') || value == "N...")
}

pub(crate) fn valid_mode(value: &str) -> bool {
    value.len() == 6 && value.bytes().all(|byte| matches!(byte, b'0'..=b'7'))
}

pub(crate) fn valid_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn valid_score(value: &str) -> bool {
    matches!(value.as_bytes().first(), Some(b'R' | b'C'))
        && value.len() > 1
        && value[1..].bytes().all(|byte| byte.is_ascii_digit())
        && value[1..].parse::<u8>().is_ok_and(|score| score <= 100)
}
