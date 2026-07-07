use crate::digest;
use crate::schema_catalog::SchemaStore;
use serde_json::Value;

pub(crate) fn validate(store: &SchemaStore, schema: &Value, instance: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    validate_at(store, schema, schema, instance, "$", &mut errors, 0);
    errors
}

pub(super) fn validate_at(
    store: &SchemaStore,
    root: &Value,
    schema: &Value,
    instance: &Value,
    path: &str,
    errors: &mut Vec<String>,
    depth: usize,
) {
    if depth > 64 {
        errors.push(format!("{path}: schema ref depth exceeded"));
        return;
    }
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        if let Some(key) = cache_key(root, reference, instance)
            && crate::schema_catalog::ref_cache_hit(store, &key)
        {
            return;
        }
        let before = errors.len();
        match resolve_ref(store, root, reference) {
            Some((doc, target)) => {
                validate_at(store, &doc, &target, instance, path, errors, depth + 1);
                if errors.len() == before
                    && let Some(key) = cache_key(root, reference, instance)
                {
                    crate::schema_catalog::cache_ref_success(store, key);
                }
            }
            None => errors.push(format!("{path}: unresolved schema ref {reference}")),
        }
        return;
    }
    structural_checks(store, root, schema, instance, path, errors, depth);
    crate::schema_catalog::schema::object::keywords::check(
        store, root, schema, instance, path, errors, depth,
    );
    crate::schema_catalog::schema::array::keywords::check(
        store, root, schema, instance, path, errors, depth,
    );
    crate::schema_catalog::schema::scalar::keywords::check(schema, instance, path, errors);
}

fn cache_key(root: &Value, reference: &str, instance: &Value) -> Option<String> {
    let root_id = root.get("$id").and_then(Value::as_str)?;
    let digest = digest::canonical_json(instance);
    Some(format!("{root_id}\n{reference}\n{digest}"))
}

fn structural_checks(
    store: &SchemaStore,
    root: &Value,
    schema: &Value,
    instance: &Value,
    path: &str,
    errors: &mut Vec<String>,
    depth: usize,
) {
    if let Some(expected) = schema.get("const")
        && instance != expected
    {
        errors.push(format!("{path}: const mismatch"));
    }
    if let Some(items) = schema.get("enum").and_then(Value::as_array)
        && !items.iter().any(|item| item == instance)
    {
        errors.push(format!("{path}: enum mismatch"));
    }
    if !type_matches(schema.get("type"), instance) {
        errors.push(format!("{path}: type mismatch"));
    }
    crate::schema_catalog::schema::scalar::keywords::number_checks(schema, instance, path, errors);
    for subschema in schema
        .get("allOf")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        validate_at(store, root, subschema, instance, path, errors, depth + 1);
    }
    any_one_of_checks(store, root, schema, instance, path, errors, depth);
    conditional_checks(store, root, schema, instance, path, errors, depth);
    not_check(store, root, schema, instance, path, errors, depth);
}

fn any_one_of_checks(
    store: &SchemaStore,
    root: &Value,
    schema: &Value,
    instance: &Value,
    path: &str,
    errors: &mut Vec<String>,
    depth: usize,
) {
    if let Some(branches) = schema.get("anyOf").and_then(Value::as_array) {
        let matches = branch_matches(store, root, branches, instance, path, depth);
        if matches == 0 {
            errors.push(format!("{path}: anyOf mismatch"));
        }
    }
    if let Some(branches) = schema.get("oneOf").and_then(Value::as_array) {
        let matches = branch_matches(store, root, branches, instance, path, depth);
        if matches != 1 {
            errors.push(format!("{path}: oneOf mismatch"));
        }
    }
}

fn conditional_checks(
    store: &SchemaStore,
    root: &Value,
    schema: &Value,
    instance: &Value,
    path: &str,
    errors: &mut Vec<String>,
    depth: usize,
) {
    let Some(if_schema) = schema.get("if") else {
        return;
    };
    let mut if_errors = Vec::new();
    validate_at(
        store,
        root,
        if_schema,
        instance,
        path,
        &mut if_errors,
        depth + 1,
    );
    let branch = if if_errors.is_empty() {
        schema.get("then")
    } else {
        schema.get("else")
    };
    if let Some(branch_schema) = branch {
        validate_at(
            store,
            root,
            branch_schema,
            instance,
            path,
            errors,
            depth + 1,
        );
    }
}

fn not_check(
    store: &SchemaStore,
    root: &Value,
    schema: &Value,
    instance: &Value,
    path: &str,
    errors: &mut Vec<String>,
    depth: usize,
) {
    if let Some(not_schema) = schema.get("not") {
        let mut nested = Vec::new();
        validate_at(
            store,
            root,
            not_schema,
            instance,
            path,
            &mut nested,
            depth + 1,
        );
        if nested.is_empty() {
            errors.push(format!("{path}: not matched forbidden schema"));
        }
    }
}

fn branch_matches(
    store: &SchemaStore,
    root: &Value,
    branches: &[Value],
    instance: &Value,
    path: &str,
    depth: usize,
) -> usize {
    branches
        .iter()
        .filter(|branch| {
            let mut errors = Vec::new();
            validate_at(store, root, branch, instance, path, &mut errors, depth + 1);
            errors.is_empty()
        })
        .count()
}

fn resolve_ref(store: &SchemaStore, root: &Value, reference: &str) -> Option<(Value, Value)> {
    let (base, fragment) = reference.split_once('#').unwrap_or((reference, ""));
    let doc = if base.is_empty() {
        root.clone()
    } else {
        store.schemas.get(base).cloned().or_else(|| {
            let name = base.rsplit('/').next().unwrap_or(base);
            store.schemas.get(name).cloned()
        })?
    };
    let target = if fragment.is_empty() {
        doc.clone()
    } else {
        doc.pointer(fragment).cloned()?
    };
    Some((doc, target))
}

fn type_matches(schema_type: Option<&Value>, instance: &Value) -> bool {
    match schema_type {
        None => true,
        Some(Value::String(kind)) => one_type_matches(kind, instance),
        Some(Value::Array(kinds)) => kinds
            .iter()
            .filter_map(Value::as_str)
            .any(|kind| one_type_matches(kind, instance)),
        _ => true,
    }
}

fn one_type_matches(kind: &str, instance: &Value) -> bool {
    match kind {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "boolean" => instance.is_boolean(),
        "integer" => instance.as_i64().is_some() || instance.as_u64().is_some(),
        "number" => instance.is_number(),
        "null" => instance.is_null(),
        _ => true,
    }
}
