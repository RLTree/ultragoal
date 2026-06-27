use serde_json::Value;

const SUBSTITUTION_TERMS: &[(&str, &str)] = &[
    ("install", "product_fitness_install_substituted_for_success"),
    (
        "first run",
        "product_fitness_first_use_substituted_for_success",
    ),
    (
        "smoke",
        "product_fitness_smoke_test_substituted_for_success",
    ),
    (
        "test pass",
        "product_fitness_test_pass_substituted_for_user_value",
    ),
    (
        "package publication",
        "product_fitness_package_publication_substituted_for_product_success",
    ),
    (
        "reviewer approved",
        "product_fitness_reviewer_substituted_for_user_evidence",
    ),
    ("opinion", "product_fitness_opinion_only"),
    (
        "feature delivered",
        "product_fitness_output_substituted_for_outcome",
    ),
    ("happy path", "product_fitness_happy_path_only"),
    (
        "fixture",
        "product_fitness_fixture_substituted_for_real_use",
    ),
    ("daily driver", "product_fitness_daily_driver_overclaim"),
    (
        "single run",
        "product_fitness_single_run_substituted_for_retention",
    ),
    (
        "entrypoint confusion",
        "product_fitness_entrypoint_confusion_unresolved",
    ),
    ("hidden power", "product_fitness_power_hidden_by_interface"),
];

pub(crate) fn failures(receipt: &Value) -> Vec<String> {
    let substitutions = receipt
        .pointer("/substitution_rejections")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    SUBSTITUTION_TERMS
        .iter()
        .filter_map(|(term, error)| {
            match substitutions.iter().find(|evidence| {
                string(evidence, "rejected_substitute")
                    .to_ascii_lowercase()
                    .contains(term)
            }) {
                Some(evidence) if string(evidence, "reason").is_empty() => {
                    Some((*error).to_string())
                }
                Some(_) => None,
                None => Some((*error).to_string()),
            }
        })
        .collect()
}

fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
