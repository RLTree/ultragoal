pub(super) fn failures(rel: &str, source: &str) -> Vec<String> {
    let bindings = super::wire_bindings::approved_literal_lines(rel, source);
    let mut failures = source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let line_number = index + 1;
            (!approved_literal_on_line(&bindings, line_number, line))
                .then(|| super::string_labels::failure(rel, line_number, line))
                .flatten()
        })
        .collect::<Vec<_>>();
    failures.extend(super::string_labels::raw_source_failures(rel, source));
    failures
}

fn approved_literal_on_line(bindings: &[(usize, String)], line_number: usize, line: &str) -> bool {
    bindings.iter().any(|(allowed_line, wire_key)| {
        *allowed_line == line_number && line.contains(&format!("\"{wire_key}\""))
    })
}
