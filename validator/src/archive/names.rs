const JUNK_ROOTS: &[&str] = &["__MACOSX", "__pycache__", ".serena"];

pub fn validate_zip_root(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err("archive zip root is empty".to_string());
    }
    if value.trim() != value {
        return Err("archive zip root has surrounding whitespace".to_string());
    }
    if value == "." || value == ".." {
        return Err(format!("archive zip root is ambiguous: {value}"));
    }
    if JUNK_ROOTS.contains(&value) {
        return Err(format!("archive zip root is junk metadata: {value}"));
    }
    if value.contains('/') || value.contains('\\') {
        return Err(format!(
            "archive zip root must be one directory name: {value}"
        ));
    }
    if value.chars().any(|ch| ch.is_control()) {
        return Err("archive zip root contains a control character".to_string());
    }
    Ok(value.to_string())
}

pub fn entry_name(zip_root: &str, rel: &str) -> Result<String, String> {
    let root = validate_zip_root(zip_root)?;
    validate_entry_rel(rel)?;
    Ok(format!("{root}/{rel}"))
}

fn validate_entry_rel(rel: &str) -> Result<(), String> {
    if rel.is_empty() {
        return Err("archive entry path is empty".to_string());
    }
    if rel.starts_with('/') || rel.ends_with('/') {
        return Err(format!("archive entry path has ambiguous slash: {rel}"));
    }
    if rel.contains('\\') || rel.contains("//") {
        return Err(format!("archive entry path has unsafe separator: {rel}"));
    }
    if rel.chars().any(|ch| ch.is_control()) {
        return Err("archive entry path contains a control character".to_string());
    }
    for part in rel.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(format!("archive entry path escapes root: {rel}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{entry_name, validate_zip_root};

    #[test]
    fn rejects_traversal_zip_roots() {
        for root in ["../escape", "/tmp/root", "safe/../root", "safe\\root", "."] {
            assert!(validate_zip_root(root).is_err(), "{root}");
        }
    }

    #[test]
    fn rejects_traversal_entry_paths() {
        for rel in [
            "../README.md",
            "docs/../README.md",
            "/README.md",
            "docs//x.md",
        ] {
            assert!(entry_name("harness-ultragoal-plugin-proposal", rel).is_err());
        }
    }

    #[test]
    fn accepts_canonical_entry_paths() {
        let name = entry_name("harness-ultragoal-plugin-proposal", "docs/index.md");
        assert_eq!(
            name,
            Ok("harness-ultragoal-plugin-proposal/docs/index.md".to_string())
        );
    }

    #[test]
    fn rejects_empty_junk_whitespace_and_control_archive_names() {
        for root in ["", " harness", "__MACOSX", "ro\not"] {
            assert!(validate_zip_root(root).is_err(), "{root:?}");
        }
        for rel in ["", "docs/", "docs\\x.md", "docs/\u{7}.md"] {
            assert!(entry_name("harness-ultragoal-plugin-proposal", rel).is_err());
        }
    }
}
