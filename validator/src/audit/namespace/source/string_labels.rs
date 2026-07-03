pub(crate) fn failure(rel: &str, line_number: usize, line: &str) -> Option<String> {
    if source_path_exempt(rel) || !line.contains('"') {
        return None;
    }
    let label = string_literals(line)
        .iter()
        .find_map(|literal| super::path_labels::product_opaque_goal_work_string_label(literal))?;
    if allowed_label_literal(rel, line, &label) {
        return None;
    }
    Some(failure_message(rel, line_number, &label))
}

pub(crate) fn raw_source_failures(rel: &str, source: &str) -> Vec<String> {
    if source_path_exempt(rel) {
        return Vec::new();
    }
    raw_string_literals(source)
        .into_iter()
        .filter_map(|literal| {
            let label = super::path_labels::product_opaque_goal_work_string_label(literal.content)?;
            if allowed_label_literal(rel, literal.content, &label) {
                return None;
            }
            Some(failure_message(rel, literal.line_number, &label))
        })
        .collect()
}

fn source_path_exempt(rel: &str) -> bool {
    rel.starts_with("validator/src/audit/namespace/")
        || rel.starts_with("validator/src/self_tests/namespace/")
}

fn allowed_label_literal(rel: &str, line: &str, label: &str) -> bool {
    owned_template_project_state_rejection_catalog(rel, line, label)
        || builder_contract_boundary_literal(rel, line, label)
        || semantic_negative_fixture_literal(rel, label)
}

fn failure_message(rel: &str, line_number: usize, label: &str) -> String {
    format!(
        "namespace_validator_source_product_opaque_goal_work_string:path={rel};line={line_number};label={label};why=string_ids_or_diagnostics_name_goal_work_or_evidence_posture_instead_of_product_behavior;repair=rename_string_id_or_diagnostic_by_cli_product_behavior_or_authority_object;claims=completion,review,package,readiness,release,product_readiness,cli_self_law,source_audit,final_packet,update_goal;exception_allowed=false"
    )
}

fn owned_template_project_state_rejection_catalog(rel: &str, line: &str, label: &str) -> bool {
    rel == "validator/src/audit/template_integrity.rs"
        && label.starts_with("session_history_")
        && line.trim_start().starts_with('"')
}

fn builder_contract_boundary_literal(rel: &str, line: &str, label: &str) -> bool {
    label == "session_history_builder_contract"
        && line.contains("parent-session-full-ultragoal")
        && matches!(
            rel,
            "validator/src/package/inventory/mod.rs"
                | "validator/src/package/inventory/tests.rs"
                | "validator/src/self_tests/package/inventory_schema_edges.rs"
                | "validator/src/self_tests/coverage/scope/authority.rs"
                | "validator/src/self_tests/coverage/scope/boundaries.rs"
                | "validator/src/self_tests/boundaries/authority/graph_edges.rs"
                | "validator/src/self_tests/boundaries/authority/inventory/roles.rs"
        )
}

fn semantic_negative_fixture_literal(rel: &str, label: &str) -> bool {
    rel == "validator/src/self_tests/boundaries/authority/inventory/labels.rs"
        && matches!(
            label,
            "gate_number"
                | "session_history_builder_contract"
                | "checkpoint"
                | "slice"
                | "fitting"
                | "observability_product_closure"
        )
}

fn string_literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for ch in line.chars() {
        if !in_string {
            if ch == '"' {
                in_string = true;
                current.clear();
            }
            continue;
        }
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            out.push(std::mem::take(&mut current));
            in_string = false;
            continue;
        }
        current.push(ch);
    }
    out
}

struct RawLiteral<'a> {
    line_number: usize,
    content: &'a str,
}

fn raw_string_literals(source: &str) -> Vec<RawLiteral<'_>> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'r' || identifier_neighbor(source, index) {
            index += 1;
            continue;
        }
        let mut cursor = index + 1;
        while cursor < bytes.len() && bytes[cursor] == b'#' {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'"' {
            index += 1;
            continue;
        }
        let hash_count = cursor - index - 1;
        let content_start = cursor + 1;
        let content_end = raw_string_content_end(source, content_start, hash_count);
        out.push(RawLiteral {
            line_number: line_number(source, index),
            content: &source[content_start..content_end],
        });
        index = if content_end < bytes.len() {
            content_end + hash_count + 1
        } else {
            bytes.len()
        };
    }
    out
}

fn identifier_neighbor(source: &str, index: usize) -> bool {
    source[..index]
        .chars()
        .next_back()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn raw_string_content_end(source: &str, start: usize, hash_count: usize) -> usize {
    let bytes = source.as_bytes();
    let mut cursor = start;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && cursor + 1 + hash_count <= bytes.len()
            && bytes[cursor + 1..cursor + 1 + hash_count]
                .iter()
                .all(|byte| *byte == b'#')
        {
            return cursor;
        }
        cursor += 1;
    }
    bytes.len()
}

fn line_number(source: &str, index: usize) -> usize {
    source[..index]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}
