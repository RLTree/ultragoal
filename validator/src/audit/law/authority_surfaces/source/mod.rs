use std::path::Path;

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
    let parent_text = std::fs::read_to_string(root.join(parent_rel)).unwrap_or_default();
    let mut previous_nonempty = "";
    for line in parent_text.lines() {
        let trimmed = line.trim();
        if trimmed == format!("mod {module_name};") && previous_nonempty == "#[cfg(test)]" {
            return true;
        }
        if !trimmed.is_empty() {
            previous_nonempty = trimmed;
        }
    }
    false
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
        if skip_depth.is_none() && pending_cfg_test && trimmed.starts_with("mod ") {
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
