pub(crate) fn failure(rel: &str, line_number: usize, line: &str) -> Option<String> {
    if !line.contains('"') {
        return None;
    }
    let label = string_literals(line)
        .iter()
        .filter(|literal| literal_has_authority_context(line, literal))
        .find_map(|literal| super::path_labels::product_opaque_goal_work_string_label(literal))?;
    if allowed_label_literal(rel, line, label) {
        return None;
    }
    Some(failure_message(rel, line_number, label))
}

pub(crate) fn raw_source_failures(rel: &str, source: &str) -> Vec<String> {
    raw_string_literals(source)
        .into_iter()
        .filter_map(|literal| {
            if super::source_shape::embedded_rust_source(literal.content) {
                return None;
            }
            if !super::source_shape::raw_literal_is_direct_authority_label(literal.content)
                && !raw_literal_has_authority_context(source, literal.start)
            {
                return None;
            }
            let label = super::path_labels::product_opaque_goal_work_string_label(literal.content)?;
            if allowed_label_literal(rel, literal.content, label) {
                return None;
            }
            Some(failure_message(rel, literal.line_number, label))
        })
        .collect()
}

fn literal_has_authority_context(line: &str, literal: &str) -> bool {
    super::path_labels::authority_like_literal(literal) || diagnostic_context(line)
}

fn raw_literal_has_authority_context(source: &str, start: usize) -> bool {
    let declaration = source[..start]
        .rsplit_once('\n')
        .map_or(&source[..start], |(_, line)| line);
    diagnostic_context(declaration)
}

fn diagnostic_context(source: &str) -> bool {
    let upper = source.to_ascii_uppercase();
    [
        "_ID",
        "_PATH",
        "_DIAGNOSTIC",
        "_ERROR",
        "_RECEIPT",
        "_STATUS",
        "_STATE",
        "_LABEL",
    ]
    .iter()
    .any(|suffix| upper.contains(suffix))
}

fn allowed_label_literal(rel: &str, line: &str, label: &str) -> bool {
    owned_template_project_state_rejection_catalog(rel, line, label)
        || builder_contract_boundary_literal(rel, line, label)
        || semantic_negative_fixture_literal(rel, label)
        || super::rejection_ownership::catalog_or_fixture_owns_label(rel, label)
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
    let raw_spans = raw_string_literals(line)
        .into_iter()
        .map(|literal| (literal.start, literal.end))
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut index = 0usize;
    while index < line.len() {
        if let Some((_, end)) = raw_spans
            .iter()
            .find(|(start, end)| *start <= index && index < *end)
        {
            index = *end;
            continue;
        }
        let Some(ch) = line[index..].chars().next() else {
            break;
        };
        if !in_string {
            if ch == '"' {
                in_string = true;
                current.clear();
            }
            index += ch.len_utf8();
            continue;
        }
        if escaped {
            current.push(ch);
            escaped = false;
            index += ch.len_utf8();
            continue;
        }
        if ch == '\\' {
            escaped = true;
            index += ch.len_utf8();
            continue;
        }
        if ch == '"' {
            out.push(std::mem::take(&mut current));
            in_string = false;
            index += ch.len_utf8();
            continue;
        }
        current.push(ch);
        index += ch.len_utf8();
    }
    out
}

struct RawLiteral<'a> {
    line_number: usize,
    start: usize,
    end: usize,
    content: &'a str,
}

fn raw_string_literals(source: &str) -> Vec<RawLiteral<'_>> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        let Some((content_start, hash_count)) = raw_string_start(source, index) else {
            index += 1;
            continue;
        };
        let content_end = raw_string_content_end(source, content_start, hash_count);
        let end = raw_literal_end(source, content_end, hash_count);
        out.push(RawLiteral {
            line_number: line_number(source, index),
            start: index,
            end,
            content: &source[content_start..content_end],
        });
        index = end;
    }
    out
}

fn raw_string_start(source: &str, index: usize) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    let prefix_end = match bytes.get(index..) {
        Some([b'b', b'r', ..]) => index + 2,
        Some([b'r', ..]) => index + 1,
        _ => return None,
    };
    if identifier_neighbor(source, index) {
        return None;
    }
    let mut cursor = prefix_end;
    while bytes.get(cursor) == Some(&b'#') {
        cursor += 1;
    }
    (bytes.get(cursor) == Some(&b'"')).then_some((cursor + 1, cursor - prefix_end))
}

fn raw_literal_end(source: &str, content_end: usize, hash_count: usize) -> usize {
    if content_end == source.len() {
        return content_end;
    }
    content_end + hash_count + 1
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
