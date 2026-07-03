use crate::schema_catalog::SchemaStore;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub fn check(
    _root: &std::path::Path,
    store: &SchemaStore,
    catalog_ids: &BTreeSet<&str>,
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    let catalog = catalog_ids
        .iter()
        .map(|id| (*id).to_string())
        .collect::<BTreeSet<_>>();
    compare(
        "schema-authority-primitives requiredRedFixtureId",
        &schema_required_red_fixture_ids(store),
        &catalog,
        failures,
    );
    compare(
        "validator-receipt red_fixtures.required",
        &receipt_schema(store),
        &catalog,
        failures,
    );
}

fn schema_required_red_fixture_ids(store: &SchemaStore) -> Option<BTreeSet<String>> {
    ids_from_array(
        store
            .schemas
            .get("schema-authority-primitives.schema.json")
            .and_then(|schema| schema.pointer("/$defs/requiredRedFixtureId/enum")),
    )
}

fn receipt_schema(store: &SchemaStore) -> Option<BTreeSet<String>> {
    ids_from_array(
        store
            .schemas
            .get("validator-receipt.schema.json")
            .and_then(|schema| schema.pointer("/properties/red_fixtures/required")),
    )
}

fn ids_from_array(value: Option<&Value>) -> Option<BTreeSet<String>> {
    Some(
        value?
            .as_array()?
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
    )
}

fn compare(
    name: &str,
    actual: &Option<BTreeSet<String>>,
    catalog: &BTreeSet<String>,
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    let Some(actual) = actual else {
        push(failures, format!("{name} unavailable"));
        return;
    };
    if actual == catalog {
        return;
    }
    let missing = catalog.difference(actual).cloned().collect::<Vec<_>>();
    let extra = actual.difference(catalog).cloned().collect::<Vec<_>>();
    push(
        failures,
        format!("{name} diverges from red catalog: missing={missing:?} extra={extra:?}"),
    );
}

fn push(failures: &mut BTreeMap<String, Vec<String>>, detail: String) {
    failures
        .entry("red-fixture-coverage".to_string())
        .or_default()
        .push(detail);
}
