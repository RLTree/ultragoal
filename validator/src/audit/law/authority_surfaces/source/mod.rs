use std::path::Path;

#[cfg(test)]
mod cfg_test_module_tests;
mod output;
mod raw;

pub(super) fn source_text_failures(root: &Path) -> Vec<(String, String)> {
    actual_source_files(root)
        .into_iter()
        .flat_map(|rel| {
            let text = std::fs::read_to_string(root.join(&rel)).unwrap_or_default();
            let text = strip_cfg_test_modules(&text);
            let mut out = raw::failures_for_text(&rel, &text)
                .into_iter()
                .map(|failure| ("typed-records-over-prose".to_string(), failure))
                .collect::<Vec<_>>();
            out.extend(
                output::failures_for_text(&rel, &text)
                    .into_iter()
                    .map(|failure| {
                        (
                            "total-authority-types-impossible-state-elimination".to_string(),
                            failure,
                        )
                    }),
            );
            out
        })
        .collect()
}

fn actual_source_files(root: &Path) -> Vec<String> {
    crate::package::inventory::closure::actual_files(root)
        .unwrap_or_default()
        .into_iter()
        .filter(|rel| rel.starts_with("validator/src/") && rel.ends_with(".rs"))
        .filter(|rel| !rel.contains("/self_tests/"))
        .filter(|rel| !rel.contains("/tests/"))
        .filter(|rel| !rel.ends_with("_tests.rs"))
        .filter(|rel| !cfg_test_module_file(root, rel))
        .collect()
}

fn cfg_test_module_file(root: &Path, rel: &str) -> bool {
    let Some((parent_rel, module_name)) = parent_module_and_name(rel) else {
        return false;
    };
    if cfg_test_path_sibling_file(root, rel, &module_name) {
        return true;
    }
    cfg_test_declared_in_parent_mod(root, &parent_rel, &module_name)
}

fn cfg_test_declared_in_parent_mod(root: &Path, parent_rel: &str, module_name: &str) -> bool {
    let parent_text = std::fs::read_to_string(root.join(parent_rel)).unwrap_or_default();
    let mut pending_cfg_test = false;
    for line in parent_text.lines() {
        let trimmed = line.trim();
        if pending_cfg_test && module_declaration_for(trimmed, &module_name) {
            return true;
        }
        if !trimmed.is_empty() {
            if trimmed == "#[cfg(test)]" {
                pending_cfg_test = true;
            } else if pending_cfg_test && trimmed.starts_with("#[") {
                continue;
            } else {
                pending_cfg_test = false;
            }
        }
    }
    false
}

fn cfg_test_path_sibling_file(root: &Path, rel: &str, module_name: &str) -> bool {
    let Some((dir, file_name)) = rel.rsplit_once('/') else {
        return false;
    };
    let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
        return false;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("rs"))
        .any(|entry| {
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str()) == Some(file_name) {
                return false;
            }
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            let sibling_file = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            let sibling_module = sibling_file.strip_suffix(".rs").unwrap_or("");
            let sibling_parent = format!("{dir}/mod.rs");
            declares_cfg_test_path_module(&text, file_name, module_name)
                || (cfg_test_declared_in_parent_mod(root, &sibling_parent, sibling_module)
                    && declares_path_module(&text, file_name, module_name))
        })
}

fn declares_path_module(text: &str, file_name: &str, module_name: &str) -> bool {
    let mut pending_path_file = false;
    let expected_path = format!("#[path = \"{file_name}\"]");
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == expected_path {
            pending_path_file = true;
            continue;
        }
        if pending_path_file && module_declaration_for(trimmed, module_name) {
            return true;
        }
        if pending_path_file && trimmed.starts_with("#[") {
            continue;
        }
        pending_path_file = false;
    }
    false
}

fn declares_cfg_test_path_module(text: &str, file_name: &str, module_name: &str) -> bool {
    let mut pending_cfg_test = false;
    let mut pending_path_file = false;
    let expected_path = format!("#[path = \"{file_name}\"]");
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "#[cfg(test)]" {
            pending_cfg_test = true;
            pending_path_file = false;
            continue;
        }
        if pending_cfg_test && trimmed == expected_path {
            pending_path_file = true;
            continue;
        }
        if pending_cfg_test && pending_path_file && module_declaration_for(trimmed, module_name) {
            return true;
        }
        if pending_cfg_test && trimmed.starts_with("#[") {
            continue;
        }
        pending_cfg_test = false;
        pending_path_file = false;
    }
    false
}

fn module_declaration_for(trimmed: &str, module_name: &str) -> bool {
    trimmed == format!("mod {module_name};")
        || trimmed == format!("pub mod {module_name};")
        || trimmed.ends_with(&format!(" mod {module_name};"))
}

fn parent_module_and_name(rel: &str) -> Option<(String, String)> {
    let (dir, file) = rel.rsplit_once('/')?;
    let name = file.strip_suffix(".rs")?;
    if name == "mod" {
        return None;
    }
    Some((format!("{dir}/mod.rs"), name.to_string()))
}

fn strip_cfg_test_modules(text: &str) -> String {
    let mut out = String::new();
    let mut skip_depth: Option<isize> = None;
    let mut pending_cfg_test = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if skip_depth.is_none() && trimmed == "#[cfg(test)]" {
            pending_cfg_test = true;
            continue;
        }
        if skip_depth.is_none() && pending_cfg_test && trimmed.starts_with("#[") {
            continue;
        }
        if skip_depth.is_none() && pending_cfg_test && module_declaration_start(trimmed) {
            if trimmed.ends_with(';') {
                pending_cfg_test = false;
                continue;
            }
            let depth = brace_delta(line);
            skip_depth = Some(depth.max(1));
            pending_cfg_test = false;
            continue;
        }
        pending_cfg_test = false;
        if let Some(depth) = skip_depth.as_mut() {
            let next = (*depth + brace_delta(line)).max(0);
            if next == 0 {
                skip_depth = None;
            } else {
                *depth = next;
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn module_declaration_start(trimmed: &str) -> bool {
    trimmed.starts_with("mod ") || trimmed.starts_with("pub mod ")
}

fn brace_delta(line: &str) -> isize {
    let opens = line.chars().filter(|ch| *ch == '{').count() as isize;
    let closes = line.chars().filter(|ch| *ch == '}').count() as isize;
    opens - closes
}

#[cfg(test)]
pub(crate) fn raw_authority_failures_for_test(rel: &str, text: &str) -> Vec<String> {
    raw::failures_for_text(rel, text)
}

#[cfg(test)]
pub(crate) fn output_authority_failures_for_test(rel: &str, text: &str) -> Vec<String> {
    output::failures_for_text(rel, text)
}
