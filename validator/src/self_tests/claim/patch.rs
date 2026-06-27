use serde_json::json;

#[test]
fn json_patch_array_authority_accepts_and_rejects_typed_operations() {
    let doc = json!({"rows":[{"id":"a"},{"id":"b"}],"nested":{"arr":[1]}});
    let patched = crate::claim_semantics::apply_patch(
        &doc,
        &json!([
            {"op":"add","path":"/rows/-","value":{"id":"c"}},
            {"op":"replace","path":"/rows/0/id","value":"aa"},
            {"op":"remove","path":"/nested/arr/0"}
        ]),
    )
    .expect("valid patch");
    assert_eq!(patched["rows"][0]["id"], "aa");
    assert_eq!(patched["rows"][2]["id"], "c");
    assert_eq!(patched["nested"]["arr"].as_array().expect("array").len(), 0);

    for bad in [
        json!({"op":"add","path":"/rows/9","value":{}}),
        json!({"op":"replace","path":"/rows/nope","value":{}}),
        json!({"op":"remove","path":"/rows/9"}),
        json!({"op":"copy","path":"/rows/0"}),
        json!({"op":"add","path":"/rows/0/id/extra","value":true}),
    ] {
        assert!(
            crate::claim_semantics::apply_patch(&doc, &json!([bad])).is_err(),
            "bad patch was accepted"
        );
    }
}
