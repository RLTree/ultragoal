pub fn direct_execution_substituted_permits(permit: &RootPermit) -> Vec<RootPermit> {
    let original = serde_json::to_value(permit).unwrap();
    [
        (
            "root_actor",
            serde_json::Value::String("other-root".to_owned()),
        ),
        (
            "binding",
            serde_json::json!({"context_id": digest('c'), "candidate_id": digest('d')}),
        ),
        (
            "target",
            serde_json::json!({
                "lease_id": "other-lease",
                "result_commitment_id": null,
                "operation_id": null,
                "recovered_binding": null
            }),
        ),
    ]
    .into_iter()
    .map(|(field, value)| {
        let mut changed = original.clone();
        changed[field] = value;
        serde_json::from_value(changed).unwrap()
    })
    .collect()
}
