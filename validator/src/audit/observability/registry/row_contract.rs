use serde_json::{Map, Value};

pub(super) fn complete(row: &Map<String, Value>) -> bool {
    [
        &["log"][..],
        &["metric"][..],
        &["trace"][..],
        &["pass", "stdout"][..],
        &["fail", "stdout"][..],
        &["receipt", "observability"][..],
    ]
    .into_iter()
    .all(|terms| row_accounts_for(row, terms))
}

fn row_accounts_for(row: &Map<String, Value>, terms: &[&str]) -> bool {
    ["fitted_surfaces", "missing_surfaces"]
        .into_iter()
        .flat_map(|key| row.get(key).and_then(Value::as_array).into_iter().flatten())
        .filter_map(Value::as_str)
        .any(|surface| {
            let lower = surface.to_ascii_lowercase();
            terms.iter().all(|term| lower.contains(term))
        })
}
