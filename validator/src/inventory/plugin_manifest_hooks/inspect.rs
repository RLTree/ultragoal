pub(super) fn inspect(reads: &ReadSession, root: &Path, value: &Value) -> Inspection {
    match value {
        Value::String(path) => super::plugin_manifest_hook_document::inspect(reads, root, path),
        Value::Object(_) => inline_inspect(value),
        Value::Array(items) if !items.is_empty() && items.len() <= MAX_LIST_ITEMS => {
            let paths = items.iter().all(Value::is_string);
            let inline = items.iter().all(Value::is_object);
            if paths {
                combine(items.iter().map(|item| {
                    super::plugin_manifest_hook_document::inspect(
                        reads,
                        root,
                        item.as_str().unwrap_or_default(),
                    )
                }))
            } else if inline {
                combine(items.iter().map(inline_inspect))
            } else {
                Inspection {
                    valid: false,
                    inactive: false,
                    trust_required: false,
                    paths: Vec::new(),
                }
            }
        }
        _ => Inspection {
            valid: false,
            inactive: false,
            trust_required: false,
            paths: Vec::new(),
        },
    }
}
