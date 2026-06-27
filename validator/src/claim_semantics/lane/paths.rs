#[derive(Clone, Copy)]
enum PathMode {
    RelativeOnly,
    AbsoluteOrRelative,
}

pub(super) fn normalize_owned(value: &str) -> Result<String, String> {
    normalize(value, PathMode::RelativeOnly)
}

pub(super) fn normalize_resource(value: &str) -> Result<String, String> {
    normalize(value, PathMode::AbsoluteOrRelative)
}

pub(crate) fn contains(parent: &str, child: &str) -> Result<bool, String> {
    let parent = normalize_resource(parent)?;
    let child = normalize_resource(child)?;
    Ok(path_contains_normalized(&parent, &child))
}

pub(super) fn contains_owned(parent: &str, child: &str) -> Result<bool, String> {
    let parent = normalize_owned(parent)?;
    let child = normalize_owned(child)?;
    Ok(path_contains_normalized(&parent, &child))
}

fn normalize(value: &str, mode: PathMode) -> Result<String, String> {
    if value.is_empty() {
        return Err("path is empty".to_string());
    }
    if value.contains('\\') || value.contains("//") {
        return Err(format!("path has unsafe separator: {value}"));
    }
    if value.chars().any(|ch| ch.is_control()) {
        return Err("path contains control character".to_string());
    }
    let absolute = value.starts_with('/');
    if absolute && matches!(mode, PathMode::RelativeOnly) {
        return Err(format!("path must be package-relative: {value}"));
    }
    let parts = value
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() || value.ends_with('/') {
        return Err(format!("path has empty segment: {value}"));
    }
    if parts.iter().any(|part| *part == "." || *part == "..") {
        return Err(format!("path contains dot segment: {value}"));
    }
    let joined = parts.join("/");
    if absolute {
        Ok(format!("/{joined}"))
    } else {
        Ok(joined)
    }
}

fn path_contains_normalized(parent: &str, child: &str) -> bool {
    parent == child || child.starts_with(&format!("{parent}/"))
}

#[cfg(test)]
mod tests {
    use super::{contains, contains_owned, normalize_owned, normalize_resource};

    #[test]
    fn rejects_dot_segments() {
        assert!(normalize_owned("fixtures/alt/../valid").is_err());
        assert!(normalize_resource("/repo/alt/..").is_err());
    }

    #[test]
    fn rejects_empty_unsafe_control_absolute_owned_and_trailing_paths() {
        assert_eq!(normalize_owned("").unwrap_err(), "path is empty");
        assert!(normalize_owned("fixtures//valid").is_err());
        assert!(normalize_owned("fixtures\\valid").is_err());
        assert_eq!(
            normalize_owned("fixtures/\nvalid").unwrap_err(),
            "path contains control character"
        );
        assert!(normalize_owned("/repo/absolute").is_err());
        assert!(normalize_owned("fixtures/valid/").is_err());
    }

    #[test]
    fn rejects_ambiguous_separators() {
        assert!(normalize_owned("fixtures//valid").is_err());
        assert!(normalize_resource(".state\\fixture").is_err());
    }

    #[test]
    fn compares_normal_paths() {
        assert_eq!(contains_owned("fixtures", "fixtures/valid"), Ok(true));
        assert_eq!(contains("/repo", "/repo/worktree"), Ok(true));
    }
}
