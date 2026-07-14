fn status(root: &Path) -> Vec<u8> {
    Command::new("git")
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ])
        .current_dir(root)
        .output()
        .expect("git status")
        .stdout
}

fn source_tree(root: &Path) -> Vec<(String, u32, Vec<u8>)> {
    let mut rows = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || entry
                    .path()
                    .strip_prefix(root)
                    .ok()
                    .and_then(|path| path.components().next())
                    .is_none_or(|component| component.as_os_str() != ".git")
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.depth() > 0)
        .map(|entry| {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("source tree path")
                .to_string_lossy()
                .into_owned();
            let metadata = fs::symlink_metadata(path).expect("source tree metadata");
            #[cfg(unix)]
            let mode = metadata.permissions().mode();
            #[cfg(not(unix))]
            let mode = 0;
            let bytes = if metadata.is_file() {
                fs::read(path).expect("source tree file")
            } else if metadata.file_type().is_symlink() {
                fs::read_link(path)
                    .expect("source tree symlink")
                    .to_string_lossy()
                    .as_bytes()
                    .to_vec()
            } else {
                Vec::new()
            };
            (relative, mode, bytes)
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}
