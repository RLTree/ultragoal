use super::*;

pub(crate) fn snapshot(root: &Path) -> Vec<(String, String)> {
    fn visit(root: &Path, path: &Path, rows: &mut Vec<(String, String)>) {
        let mut entries = fs::read_dir(path)
            .unwrap()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let metadata = fs::symlink_metadata(&path).unwrap();
            let value = if metadata.is_dir() {
                visit(root, &path, rows);
                "dir".to_string()
            } else if metadata.is_file() {
                format!(
                    "file:{}",
                    super::super::repository_fit::digest(&fs::read(&path).unwrap())
                )
            } else if metadata.file_type().is_symlink() {
                format!("symlink:{}", fs::read_link(&path).unwrap().display())
            } else {
                "special".to_string()
            };
            rows.push((relative, value));
        }
    }
    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows.sort();
    rows
}
