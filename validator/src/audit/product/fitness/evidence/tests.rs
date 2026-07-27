use serde_json::json;

#[test]
fn repository_evidence_uses_the_same_file_digest_boundary() {
    let root = std::env::temp_dir().join(format!(
        "ultragoal-product-fitness-evidence-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let value = json!({
        "real_work_observation": {
            "repository_evidence": {
                "path": "missing.json",
                "digest": format!("sha256:{}", "a".repeat(64))
            }
        }
    });
    let mut failures = Vec::new();
    super::failures(&root, &value, "", &mut failures);
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(
        failures,
        ["product_fitness_evidence_missing:/real_work_observation/repository_evidence"]
    );
}
