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
        let mut enum_depth: Option<usize> = None;
        let mut brace_depth = 0usize;
        for (line_index, line) in source.lines().enumerate() {
            let line_number = line_index + 1;
            out.extend(declaration_failure(rel, line_number, line));
            out.extend(parameter_failures(rel, line_number, line));
            out.extend(field_or_parameter_failure(rel, line_number, line));
            out.extend(local_binding_failure(rel, line_number, line));
            out.extend(super::string_labels::failure(rel, line_number, line));
            out.extend(enum_variant_failure(
                rel,
                line_number,
                line,
                enum_depth.is_some(),
            ));
            update_enum_depth(line, &mut enum_depth, &mut brace_depth);
        }
        out.extend(super::string_labels::raw_source_failures(rel, &source));
    }
    out
}

fn declaration_failure(rel: &str, line_number: usize, line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let public_trimmed = strip_declaration_modifiers(strip_visibility(trimmed));
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
    let public_trimmed = strip_declaration_modifiers(strip_visibility(trimmed));
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
        .or_else(|| {
            line.strip_prefix("pub(in ")
                .and_then(|rest| rest.find(") ").map(|end| &rest[end + 2..]))
        })
        .unwrap_or(line)
}

fn strip_declaration_modifiers(mut line: &str) -> &str {
    loop {
        let next = line
            .strip_prefix("async ")
            .or_else(|| line.strip_prefix("const "))
            .or_else(|| line.strip_prefix("unsafe "))
            .or_else(|| {
                line.strip_prefix("extern ")
                    .and_then(|rest| rest.find(' ').map(|end| &rest[end + 1..]))
            });
        let Some(stripped) = next else {
            return line;
        };
        line = stripped;
    }
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
    let (check_id, label, why, repair) = if let Some(label) =
        super::path_labels::product_opaque_goal_work_label(identifier)
    {
        (
            "namespace_validator_source_product_opaque_goal_work_identifier",
            label,
            "identifier_names_goal_work_or_evidence_posture_instead_of_product_behavior",
            "rename_identifier_by_cli_product_behavior_such_as_command_roundtrip_command_inventory_or_telemetry_reconciliation",
        )
    } else if let Some(label) = super::path_labels::generic_identifier_bucket_label(identifier) {
        (
            "namespace_validator_source_generic_identifier",
            label,
            "identifier_names_generic_bucket_or_helper_posture_instead_of_product_behavior",
            "rename_identifier_by_the_product_behavior_or_domain_contract_it_serves",
        )
    } else {
        return None;
    };
    Some(format!(
        "{check_id}:path={rel};line={line_number};identifier={identifier};label={label};why={why};repair={repair};claims=completion,review,package,readiness,release,product_readiness,cli_self_law,source_audit,final_packet,update_goal;exception_allowed=false"
    ))
}

fn enum_variant_failure(
    rel: &str,
    line_number: usize,
    line: &str,
    inside_enum: bool,
) -> Option<String> {
    if !inside_enum {
        return None;
    }
    let trimmed = line.trim_start();
    if trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with('}')
        || trimmed.starts_with("impl ")
    {
        return None;
    }
    let identifier = take_identifier(trimmed)?;
    if identifier
        .chars()
        .next()
        .is_some_and(|ch| !ch.is_ascii_uppercase())
    {
        return None;
    }
    identifier_failure(rel, line_number, identifier)
}

fn update_enum_depth(line: &str, enum_depth: &mut Option<usize>, brace_depth: &mut usize) {
    let trimmed = line.trim_start();
    let starts_enum = strip_declaration_modifiers(strip_visibility(trimmed)).starts_with("enum ");
    for ch in line.chars() {
        match ch {
            '{' => {
                *brace_depth += 1;
                if starts_enum && enum_depth.is_none() {
                    *enum_depth = Some(*brace_depth);
                }
            }
            '}' => {
                if enum_depth.is_some_and(|depth| depth == *brace_depth) {
                    *enum_depth = None;
                }
                *brace_depth = brace_depth.saturating_sub(1);
            }
            _ => {}
        }
    }
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
