pub(crate) mod catalog_refs;
pub(crate) mod fixture_schema_rules;
mod product;
mod receipt_schema_rules;
mod schema;

use crate::json_boundary;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct SchemaStore {
    pub schemas: BTreeMap<String, Value>,
    pub errors: Vec<String>,
    ref_success_cache: Arc<Mutex<BTreeSet<String>>>,
}

pub fn load(root: &Path) -> SchemaStore {
    let mut schemas = BTreeMap::new();
    let mut errors = Vec::new();
    let catalog_rel = "schemas/schema-catalog.json";
    let catalog = match read_catalog(root, catalog_rel, &mut errors) {
        Some(value) => value,
        None => return store(schemas, errors),
    };
    let Some(rows) = catalog.get("schemas").and_then(Value::as_array) else {
        errors.push("schema-catalog schemas must be a list".to_string());
        return store(schemas, errors);
    };
    errors.extend(catalog_refs::catalog_completeness_errors(root, rows));
    load_rows(root, rows, &mut schemas, &mut errors);
    let allowed = schemas.keys().cloned().collect::<BTreeSet<_>>();
    for (name, schema) in &schemas {
        if !name.starts_with("https://") {
            errors.extend(
                catalog_refs::ref_errors(schema, &allowed)
                    .into_iter()
                    .map(|err| format!("{name}: {err}")),
            );
            errors.extend(schema::supported::keywords::errors(name, schema));
        }
    }
    store(schemas, errors)
}

fn store(schemas: BTreeMap<String, Value>, errors: Vec<String>) -> SchemaStore {
    SchemaStore {
        schemas,
        errors,
        ref_success_cache: Arc::new(Mutex::new(BTreeSet::new())),
    }
}

pub(super) fn ref_cache_hit(store: &SchemaStore, key: &str) -> bool {
    store
        .ref_success_cache
        .lock()
        .expect("schema ref cache")
        .contains(key)
}

pub(super) fn cache_ref_success(store: &SchemaStore, key: String) {
    store
        .ref_success_cache
        .lock()
        .expect("schema ref cache")
        .insert(key);
}

pub fn schema_errors(store: &SchemaStore, schema_name: &str, instance: &Value) -> Vec<String> {
    let Some(schema) = store.schemas.get(schema_name) else {
        return vec![format!(
            "offline_schema_resolution_failed: missing schema {schema_name}"
        )];
    };
    let mut errors = schema::keywords::validate(store, schema, instance);
    errors.extend(match schema_name {
        "fixture-bundle.schema.json" => fixture_schema_rules::fixture_bundle_errors(instance),
        "red-packet.schema.json" => fixture_schema_rules::red_packet_errors(instance),
        "red-fixtures-catalog.schema.json" => fixture_schema_rules::red_catalog_errors(instance),
        "validator-receipt.schema.json" => receipt_schema_rules::validator_receipt_errors(instance),
        "target-repo-receipt.schema.json" => {
            receipt_schema_rules::target_receipt_schema_errors(instance)
        }
        "semantic-classification-receipt.schema.json" => {
            receipt_schema_rules::semantic_receipt_errors(instance)
        }
        "review-round-receipt.schema.json" => generic_required_schema_errors(instance),
        "product-cohesion-receipt.schema.json" => {
            product::cohesion::rules::product_cohesion_errors(instance)
        }
        _ => generic_required_schema_errors(instance),
    });
    errors
}

pub fn schema_error_code(errors: &[String]) -> String {
    let joined = errors.join("\n");
    if joined.contains("required_claim_ids") && joined.contains("non-empty") {
        "required_claim_missing"
    } else if joined.contains("plugin_manifest.skills") && joined.contains("missing required") {
        "required_skill_missing"
    } else if is_plugin_agent_schema_error(&joined) {
        "plugin_agent_path_missing"
    } else if joined.contains("verification_backlog.rows[0].attempts") {
        "blocker_without_attempt_evidence"
    } else if joined.contains("required_check_ids") && joined.contains("duplicate") {
        "duplicate_validator_check_rows"
    } else if joined.contains("required_red_fixture_ids") && joined.contains("duplicate") {
        "duplicate_red_fixture_rows"
    } else if joined.contains("validator_receipt.commit") && joined.contains("sha256") {
        "validator_commit_value_not_sha"
    } else if joined.contains("validator_receipt.target_revision.value")
        && joined.contains("sha256")
    {
        "package_digest_value_not_sha"
    } else if joined.contains("commands") && joined.contains("exit") && joined.contains("const") {
        "command_receipt_missing_or_failed"
    } else if joined.contains("worktree_clean") && joined.contains("const") {
        "dirty_self_reported_clean"
    } else if joined.contains("teardown_ready") && joined.contains("const") {
        "stale_worktree_after_closeout"
    } else if joined.contains("blocked_reasons") && joined.contains("maxItems") {
        "ready_receipt_has_blockers"
    } else if joined.contains("actor_binding.validated_at") && joined.contains("format date-time") {
        "actor_validation_timestamp_malformed"
    } else if joined.contains("last_heartbeat") && joined.contains("format date-time") {
        "lane_heartbeat_timestamp_malformed"
    } else if joined.contains("actor_status") && joined.contains("const") {
        "stale_actor_identity"
    } else if joined.contains("semantic_classification_receipts")
        || joined.contains("prompt_contract_digest")
        || joined.contains("classifier_evidence")
    {
        "semantic_classification_receipt_malformed"
    } else if joined.contains("root_verification_phases.final_all_lanes_gate") {
        "root_verification_stage_missing_or_duplicate"
    } else {
        "schema_validation_failed"
    }
    .to_string()
}

fn is_plugin_agent_schema_error(joined: &str) -> bool {
    let mentions_agents = joined.contains("plugin_manifest.agents")
        || joined.contains("$.agents")
        || joined.contains(".agents[");
    mentions_agents
        && (joined.contains("missing required")
            || joined.contains("contains mismatch")
            || joined.contains("const mismatch"))
}

pub fn product_cohesion_receipt_errors(instance: &Value) -> Vec<String> {
    product::cohesion::rules::product_cohesion_errors(instance)
}

fn read_catalog(root: &Path, rel: &str, errors: &mut Vec<String>) -> Option<Value> {
    if let Some(error) = crate::package::inventory::package_path_error(root, rel) {
        errors.push(format!("schema-catalog: {error}"));
        return None;
    }
    match json_boundary::read_json(&root.join(rel)) {
        Ok(value) => Some(value),
        Err(err) => {
            errors.push(format!("schema-catalog load failed: {err}"));
            None
        }
    }
}

fn load_rows(
    root: &Path,
    rows: &[Value],
    schemas: &mut BTreeMap<String, Value>,
    errors: &mut Vec<String>,
) {
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            errors.push(format!("schema row invalid: {row}"));
            continue;
        };
        let Some(path) = row.get("path").and_then(Value::as_str) else {
            errors.push(format!("schema row invalid: {row}"));
            continue;
        };
        load_one_schema(root, id, path, schemas, errors);
    }
}

fn load_one_schema(
    root: &Path,
    id: &str,
    rel: &str,
    schemas: &mut BTreeMap<String, Value>,
    errors: &mut Vec<String>,
) {
    if let Some(error) = crate::package::inventory::package_path_error(root, rel) {
        errors.push(format!("{id}: {error}"));
        return;
    }
    let schema = match json_boundary::read_json(&root.join(rel)) {
        Ok(value) => value,
        Err(err) => {
            errors.push(format!("{id}: schema load failed: {err}"));
            return;
        }
    };
    if schema.get("$id").and_then(Value::as_str) != Some(id) {
        errors.push(format!("{id}: schema id mismatch"));
        return;
    }
    if let Some(name) = Path::new(rel).file_name().and_then(|p| p.to_str()) {
        schemas.insert(name.to_string(), schema.clone());
    }
    schemas.insert(id.to_string(), schema);
}

fn generic_required_schema_errors(value: &Value) -> Vec<String> {
    if !value.is_object() && !value.is_array() {
        vec!["schema instance must be object or array".to_string()]
    } else {
        Vec::new()
    }
}
