use std::path::Path;

pub(super) fn generated_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    collect_json_files(root, &root.join("docs/generated"), &mut out);
    collect_json_files(root, &root.join("examples/generated"), &mut out);
    out.sort();
    out
}

fn collect_json_files(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(root, &path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json")
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}
