use super::transaction_observation::HostLifecycleObservationInput;
use crate::distribution::PackageIdentity;
use crate::plugin_product::lifecycle::{HostLifecycleExpectedObservations, LifecyclePlan};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(crate) fn expected_observations(
    package: &PackageIdentity,
    plan: &LifecyclePlan,
    input: &HostLifecycleObservationInput,
    command_count: usize,
) -> HostLifecycleExpectedObservations {
    HostLifecycleExpectedObservations {
        installed_sha256: identity(
            "installed",
            package,
            plan,
            input,
            plan.expected_after.installed.as_ref(),
        ),
        cache_sha256: identity(
            "cache",
            package,
            plan,
            input,
            plan.expected_after.cache.as_ref(),
        ),
        registry_sha256: identity(
            "registry",
            package,
            plan,
            input,
            plan.expected_after.installed.as_ref(),
        ),
        discovery_sha256: identity(
            "discovery",
            package,
            plan,
            input,
            plan.expected_after.installed.as_ref(),
        ),
        runtime_sha256: identity(
            "runtime",
            package,
            plan,
            input,
            plan.expected_after.installed.as_ref(),
        ),
        command_count,
    }
}

pub(crate) fn identity(
    surface: &str,
    package: &PackageIdentity,
    plan: &LifecyclePlan,
    input: &HostLifecycleObservationInput,
    expected_authority: Option<&crate::plugin_product::lifecycle::PackageAuthority>,
) -> String {
    let value = (
        "harness-ultragoal.host-lifecycle-surface.v1",
        surface,
        package,
        &plan.plan_id,
        &input.marketplace,
        &input.plugin,
        expected_authority,
    );
    format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&value).unwrap_or_default())
    )
}

pub(crate) fn normalized_json(value: &Value, needle: &str) -> Option<String> {
    if !contains_string(value, needle) {
        return None;
    }
    let mut values = Vec::new();
    flatten(value, &mut values);
    values.sort();
    Some(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&values).unwrap_or_default())
    ))
}

fn flatten(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                output.push(key.clone());
                flatten(value, output);
            }
        }
        Value::Array(values) => values.iter().for_each(|value| flatten(value, output)),
        Value::String(value) => output.push(value.clone()),
        Value::Number(value) => output.push(value.to_string()),
        Value::Bool(value) => output.push(value.to_string()),
        Value::Null => output.push("null".to_owned()),
    }
}

fn contains_string(value: &Value, needle: &str) -> bool {
    match value {
        Value::String(value) => value == needle,
        Value::Array(values) => values.iter().any(|value| contains_string(value, needle)),
        Value::Object(values) => values.values().any(|value| contains_string(value, needle)),
        _ => false,
    }
}

pub(crate) fn semantic_digest(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| digest(b"absent"))
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
