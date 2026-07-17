use std::fs;
use std::path::{Component, Path, PathBuf};

pub fn is_regular_file(repo: &Path, rel: &str) -> bool {
    regular_path(repo, rel).is_ok()
}

pub fn is_nonempty_file(repo: &Path, rel: &str) -> bool {
    file_metadata(repo, rel).is_ok_and(|meta| meta.len() > 0)
}

pub fn is_dir(repo: &Path, rel: &str) -> bool {
    dir_path(repo, rel).is_ok()
}

pub fn read_to_string(repo: &Path, rel: &str) -> Result<String, String> {
    let path = regular_path(repo, rel)?;
    String::from_utf8(crate::digest::read_file_bytes(&path)?)
        .map_err(|err| format!("{rel}: utf8 decode failed: {err}"))
}

pub fn read(repo: &Path, rel: &str) -> Result<Vec<u8>, String> {
    let path = regular_path(repo, rel)?;
    crate::digest::read_file_bytes(&path)
}

fn file_metadata(repo: &Path, rel: &str) -> Result<fs::Metadata, String> {
    regular_path(repo, rel).and_then(|path| metadata_result(rel, fs::metadata(&path)))
}

pub fn regular_path(repo: &Path, rel: &str) -> Result<PathBuf, String> {
    let path = contained(repo, rel)?;
    let meta = fs::symlink_metadata(&path)
        .map_err(|err| format!("{rel}: target path unreadable: {err}"))?;
    if meta.file_type().is_symlink() || !meta.file_type().is_file() {
        return Err(format!("{rel}: target path is not a regular in-repo file"));
    }
    Ok(path)
}

pub fn dir_path(repo: &Path, rel: &str) -> Result<PathBuf, String> {
    let path = contained(repo, rel)?;
    let meta = fs::symlink_metadata(&path)
        .map_err(|err| format!("{rel}: target path unreadable: {err}"))?;
    if meta.file_type().is_symlink() || !meta.file_type().is_dir() {
        return Err(format!(
            "{rel}: target path is not a regular in-repo directory"
        ));
    }
    Ok(path)
}

fn contained(repo: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute()
        || rel_path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("{rel}: target path escapes repo"));
    }
    let root = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    let joined = root.join(rel_path);
    reject_symlink_components(&root, rel_path, rel)?;
    let canonical = joined.canonicalize().unwrap_or(joined);
    ensure_contained(&root, &canonical, rel)?;
    Ok(canonical)
}

fn reject_symlink_components(root: &Path, rel_path: &Path, rel: &str) -> Result<(), String> {
    let mut cursor = root.to_path_buf();
    for name in rel_path.iter() {
        cursor.push(name);
        let Ok(meta) = fs::symlink_metadata(&cursor) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            return Err(format!("{rel}: target path uses symlink"));
        }
    }
    Ok(())
}

fn ensure_contained(root: &Path, canonical: &Path, rel: &str) -> Result<(), String> {
    if canonical.strip_prefix(root).is_err() {
        return Err(format!("{rel}: target path escapes repo"));
    }
    Ok(())
}

pub(crate) fn metadata_result(
    rel: &str,
    result: std::io::Result<fs::Metadata>,
) -> Result<fs::Metadata, String> {
    result.map_err(|err| format!("{rel}: metadata failed: {err}"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn contained_guard_reports_outside_target_repo() {
        let root = Path::new("/target/root");
        assert!(super::ensure_contained(root, Path::new("/target/root/file"), "file").is_ok());
        assert!(
            super::ensure_contained(root, Path::new("/outside/file"), "file")
                .expect_err("outside path rejected")
                .contains("target path escapes repo")
        );
        assert!(
            super::metadata_result(
                "file",
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "forced")),
            )
            .expect_err("metadata failure mapped")
            .contains("metadata failed")
        );
    }
}
