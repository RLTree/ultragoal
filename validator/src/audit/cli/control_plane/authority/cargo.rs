use std::collections::BTreeSet;

pub(super) fn bin_names(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut in_bin = false;
    for line in text.lines().map(str::trim) {
        match line {
            "[[bin]]" => {
                in_bin = true;
                continue;
            }
            _ if line.starts_with('[') => {
                in_bin = false;
                continue;
            }
            _ => {}
        }
        if in_bin && let Some(name) = quoted_value(line, "name") {
            out.insert(name);
        }
    }
    out
}

fn quoted_value(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    Some(rest.strip_prefix('"')?.split('"').next()?.to_string())
}
