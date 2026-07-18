use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Component, Path, PathBuf};

const REGISTRY: &str = "migration/generated-surface-authority.json";

pub(super) fn copy_declared_files(live: &Path, root: &Path) {
    let live = fs::canonicalize(live).unwrap();
    let registry_path = copy_file(&live, root, REGISTRY).unwrap();
    let registry: Value = serde_json::from_slice(&fs::read(&registry_path).unwrap()).unwrap();
    let mut paths = BTreeSet::new();
    let projection = registry.get("registry_projection").unwrap();
    insert_path(projection.get("generator"), &mut paths);
    let shard_paths = paths_from(projection.get("canonical_sources"));
    paths.extend(shard_paths.iter().cloned());
    for shard in shard_paths {
        let value: Value =
            serde_json::from_slice(&fs::read(source_file(&live, &shard).unwrap()).unwrap())
                .unwrap();
        for surface in value.get("surfaces").unwrap().as_array().unwrap() {
            insert_path(surface.get("output"), &mut paths);
            insert_path(surface.get("generator"), &mut paths);
            insert_paths(surface.get("canonical_sources"), &mut paths);
        }
    }
    for path in paths {
        copy_file(&live, root, &path).unwrap();
    }
}

fn copy_file(live: &Path, root: &Path, path: &str) -> io::Result<PathBuf> {
    let source = source_file(live, path)?;
    let relative = normal_relative(path)?;
    let root_metadata = fs::symlink_metadata(root)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(invalid("fixture authority root must be a directory"));
    }
    let canonical_root = fs::canonicalize(root)?;
    let target_parent = root.join(&relative).parent().unwrap().to_path_buf();
    fs::create_dir_all(&target_parent)?;
    let canonical_parent = fs::canonicalize(target_parent)?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(invalid(
            "fixture authority destination escapes fixture root",
        ));
    }
    let target = canonical_parent.join(relative.file_name().unwrap());
    fs::copy(source, &target)?;
    Ok(target)
}

fn source_file(live: &Path, path: &str) -> io::Result<PathBuf> {
    let relative = normal_relative(path)?;
    let source = live.join(relative);
    let metadata = fs::symlink_metadata(&source)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(invalid("fixture authority source must be a regular file"));
    }
    let canonical_source = fs::canonicalize(&source)?;
    if !canonical_source.starts_with(live) {
        return Err(invalid("fixture authority source escapes live root"));
    }
    Ok(canonical_source)
}

fn normal_relative(path: &str) -> io::Result<PathBuf> {
    let relative = Path::new(path);
    if path.is_empty()
        || !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(invalid(
            "fixture authority path must be nonempty and relative",
        ));
    }
    Ok(relative.to_path_buf())
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(ErrorKind::InvalidInput, message)
}

fn insert_path(value: Option<&Value>, paths: &mut BTreeSet<String>) {
    if let Some(path) = value.and_then(Value::as_str) {
        paths.insert(path.to_owned());
    }
}

fn insert_paths(value: Option<&Value>, paths: &mut BTreeSet<String>) {
    paths.extend(paths_from(value));
}

fn paths_from(value: Option<&Value>) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for path in value.and_then(Value::as_array).into_iter().flatten() {
        insert_path(Some(path), &mut paths);
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::{REGISTRY, copy_declared_files, copy_file, normal_relative};
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn root(label: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ultragoal-authority-inputs-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn rejects_absolute_and_parent_paths() {
        assert!(normal_relative("/outside").is_err());
        assert!(normal_relative("../outside").is_err());
        assert!(normal_relative("").is_err());
    }

    #[test]
    fn rejects_absolute_and_parent_shards_before_reading_them() {
        for (label, shard) in [
            ("absolute", std::env::temp_dir().join("outside.json")),
            ("parent", Path::new("../outside.json").to_path_buf()),
        ] {
            let live = root(&format!("shard-live-{label}"));
            let fixture = root(&format!("shard-fixture-{label}"));
            fs::create_dir_all(live.join("migration")).unwrap();
            fs::write(
                live.join(REGISTRY),
                format!(
                    r#"{{"registry_projection":{{"canonical_sources":["{}"]}},"surfaces":[]}}"#,
                    shard.display()
                ),
            )
            .unwrap();
            assert!(std::panic::catch_unwind(|| copy_declared_files(&live, &fixture)).is_err());
            assert!(!fixture.join("outside.json").exists());
            fs::remove_dir_all(live).unwrap();
            fs::remove_dir_all(fixture).unwrap();
        }
    }

    #[test]
    fn copies_a_valid_regular_file_within_fixture_root() {
        let live = root("valid-live");
        let fixture = root("valid-fixture");
        fs::create_dir_all(live.join("nested")).unwrap();
        fs::write(live.join("nested/input.json"), b"valid").unwrap();
        let target = copy_file(
            &fs::canonicalize(&live).unwrap(),
            &fixture,
            "nested/input.json",
        )
        .unwrap();
        assert_eq!(fs::read(target).unwrap(), b"valid");
        fs::remove_dir_all(live).unwrap();
        fs::remove_dir_all(fixture).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlink_to_an_outside_source() {
        use std::os::unix::fs::symlink;

        let live = root("link-live");
        let fixture = root("link-fixture");
        let outside_root = root("link-outside");
        let outside = outside_root.join("authority.json");
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, live.join("escape.json")).unwrap();
        assert!(copy_file(&fs::canonicalize(&live).unwrap(), &fixture, "escape.json").is_err());
        assert!(!fixture.join("escape.json").exists());
        fs::remove_dir_all(live).unwrap();
        fs::remove_dir_all(fixture).unwrap();
        fs::remove_dir_all(outside_root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_destination_intermediate_symlink() {
        use std::os::unix::fs::symlink;

        let live = root("destination-live");
        let fixture = root("destination-fixture");
        let outside = root("destination-outside");
        fs::create_dir_all(live.join("nested")).unwrap();
        fs::write(live.join("nested/input.json"), b"valid").unwrap();
        symlink(&outside, fixture.join("nested")).unwrap();
        assert!(
            copy_file(
                &fs::canonicalize(&live).unwrap(),
                &fixture,
                "nested/input.json"
            )
            .is_err()
        );
        assert!(!outside.join("input.json").exists());
        fs::remove_dir_all(live).unwrap();
        fs::remove_dir_all(fixture).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn normal_paths_remain_relative() {
        assert_eq!(
            normal_relative("nested/input.json").unwrap(),
            Path::new("nested/input.json")
        );
    }
}
