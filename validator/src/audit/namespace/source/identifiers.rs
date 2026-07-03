use std::path::Path;

const DECLARATION_PREFIXES: &[&str] = &[
    "fn ", "mod ", "struct ", "enum ", "trait ", "type ", "const ", "static ",
];

pub(crate) fn failures(root: &Path, actual_files: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for rel in actual_files
        .iter()
        .filter(|path| is_validator_rust_source(path))
    {
        let Ok(source) = std::fs::read_to_string(root.join(rel)) else {
            continue;
        };
        for (line_index, line) in source.lines().enumerate() {
            let line_number = line_index + 1;
            out.extend(declaration_failure(rel, line_number, line));
            out.extend(parameter_failures(rel, line_number, line));
            out.extend(field_or_parameter_failure(rel, line_number, line));
            out.extend(local_binding_failure(rel, line_number, line));
        }
    }
    out
}

fn declaration_failure(rel: &str, line_number: usize, line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let public_trimmed = strip_visibility(trimmed);
    for prefix in DECLARATION_PREFIXES {
        let Some(rest) = public_trimmed.strip_prefix(prefix) else {
            continue;
        };
        let identifier = take_identifier(rest)?;
        return identifier_failure(rel, line_number, identifier);
    }
    None
}

fn parameter_failures(rel: &str, line_number: usize, line: &str) -> Vec<String> {
    let trimmed = line.trim_start();
    let public_trimmed = strip_visibility(trimmed);
    let Some(rest) = public_trimmed.strip_prefix("fn ") else {
        return Vec::new();
    };
    let Some(open) = rest.find('(') else {
        return Vec::new();
    };
    let Some(close) = rest[open + 1..].find(')') else {
        return Vec::new();
    };
    rest[open + 1..open + 1 + close]
        .split(',')
        .filter_map(parameter_identifier)
        .filter_map(|identifier| identifier_failure(rel, line_number, identifier))
        .collect()
}

fn field_or_parameter_failure(rel: &str, line_number: usize, line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with('"') {
        return None;
    }
    let public_trimmed = strip_visibility(trimmed);
    let identifier = parameter_identifier(public_trimmed)?;
    identifier_failure(rel, line_number, identifier)
}

fn local_binding_failure(rel: &str, line_number: usize, line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("let mut ")
        .or_else(|| trimmed.strip_prefix("let "))?;
    let identifier = take_identifier(rest)?;
    identifier_failure(rel, line_number, identifier)
}

fn strip_visibility(line: &str) -> &str {
    line.strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub(super) "))
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line)
}

fn parameter_identifier(segment: &str) -> Option<&str> {
    let candidate = segment
        .trim_start()
        .strip_prefix("mut ")
        .unwrap_or(segment.trim_start());
    let colon_index = candidate.find(':')?;
    let before_colon = candidate[..colon_index].trim();
    if matches!(before_colon, "" | "_" | "self" | "&self" | "&mut self") {
        return None;
    }
    take_identifier(before_colon)
}

fn identifier_failure(rel: &str, line_number: usize, identifier: &str) -> Option<String> {
    let label = super::path_labels::product_opaque_goal_work_label(identifier)?;
    Some(format!(
        "namespace_validator_source_product_opaque_goal_work_identifier:path={rel};line={line_number};identifier={identifier};label={label};why=identifier_names_goal_work_or_evidence_posture_instead_of_product_behavior;repair=rename_identifier_by_cli_product_behavior_such_as_command_roundtrip_command_inventory_or_telemetry_reconciliation;claims=completion,review,package,readiness,release,product_readiness,cli_self_law,source_audit,final_packet,update_goal;exception_allowed=false"
    ))
}

fn take_identifier(rest: &str) -> Option<&str> {
    let start = rest.trim_start();
    let start = start.strip_prefix("r#").unwrap_or(start);
    let len = start
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_alphanumeric() || *ch == '_')
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    Some(&start[..len])
}

fn is_validator_rust_source(path: &str) -> bool {
    (path.starts_with("validator/src/") || path.starts_with("validator/tests/"))
        && path.ends_with(".rs")
}
