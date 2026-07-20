fn copy_source_context_refs(repo: &TestRepo) {
    let root = live_root();
    let product_schema = "schemas/product-success-contract.schema.json";
    repo.write(
        product_schema,
        &fs::read(root.join(product_schema)).unwrap(),
    );
    let registry: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("LANE_REGISTRY.json")).unwrap()).unwrap();
    for reference in registry["source_context"]["refs"]
        .as_object()
        .unwrap()
        .values()
    {
        copy_authority_ref(repo, &root, reference);
    }
    for reference in registry["root_freeze"]["payload_refs"].as_array().unwrap() {
        copy_authority_ref(repo, &root, reference);
    }
}

fn copy_authority_ref(repo: &TestRepo, root: &std::path::Path, reference: &serde_json::Value) {
    let Some(path) = reference.get("path").and_then(serde_json::Value::as_str) else {
        return;
    };
    let source = root.join(path);
    if source.is_file() {
        repo.write(path, &fs::read(source).unwrap());
    }
}
