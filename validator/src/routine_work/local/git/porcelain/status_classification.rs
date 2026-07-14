use super::*;

pub(crate) fn kind_from_xy(xy: &str) -> ChangeKind {
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

pub(crate) fn validate_unique_paths(rows: &[StatusRow]) -> Result<(), RoutineError> {
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

pub(crate) fn parse_path(bytes: &[u8]) -> Result<RepoPath, RoutineError> {
    let text = std::str::from_utf8(bytes).map_err(|_| capture_error("git-status-non-utf8"))?;
    RepoPath::parse(text.to_owned())
}

pub(crate) fn capture_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CaptureFailed, cause, None)
}

pub(crate) fn limit_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::CaptureLimit, cause, None)
}

pub(crate) fn invalid_snapshot(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidSnapshot, cause, None)
}

#[cfg(test)]
mod tests {
    use super::super::*;

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
