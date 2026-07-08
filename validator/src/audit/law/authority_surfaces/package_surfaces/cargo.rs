use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn bins(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(root.join("validator/Cargo.toml")) else {
        return out;
    };
    let mut in_bin = false;
    let mut name: Option<String> = None;
    let mut path: Option<String> = None;
    for line in text.lines().map(str::trim) {
        if line == "[[bin]]" {
            insert_bin(&mut out, name.take(), path.take());
            in_bin = true;
            continue;
        }
        if !in_bin {
            continue;
        }
        if line.starts_with('[') {
            insert_bin(&mut out, name.take(), path.take());
            in_bin = false;
            continue;
        }
        if let Some(value) = quoted_value(line, "name") {
            name = Some(value);
        }
        if let Some(value) = quoted_value(line, "path") {
            path = Some(format!("validator/{value}"));
        }
    }
    insert_bin(&mut out, name, path);
    out
}

fn insert_bin(out: &mut BTreeMap<String, String>, name: Option<String>, path: Option<String>) {
    if let (Some(name), Some(path)) = (name, path) {
        out.insert(name, path);
    }
}

fn quoted_value(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    Some(rest.strip_prefix('"')?.split('"').next()?.to_string())
}
