use serde_json::json;

#[test]
fn red_fixture_schema_routing_rejects_schema_layer_and_unknown_patch_roots() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let store = crate::schema_catalog::load(&root);
    let bad = json!({"completion_manifest":null});

    let schema_errors = crate::red::fixture::schema::errors(
        &store,
        &json!({"materialization":{"expected_validation_layer":"schema"}}),
        &bad,
    );
    assert!(!schema_errors.is_empty());

    let missing_patch_errors = crate::red::fixture::schema::errors(&store, &json!({}), &bad);
    assert!(!missing_patch_errors.is_empty());

    let empty_root_errors = crate::red::fixture::schema::errors(
        &store,
        &json!({"json_patch":[{"op":"remove","path":"/"}]}),
        &bad,
    );
    assert!(!empty_root_errors.is_empty());

    let unknown_root_errors = crate::red::fixture::schema::errors(
        &store,
        &json!({"json_patch":[{"op":"replace","path":"/unknown/value"}]}),
        &bad,
    );
    assert!(!unknown_root_errors.is_empty());
}

#[test]
fn red_fixture_schema_routes_every_typed_patch_root_fail_closed() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let store = crate::schema_catalog::load(&root);
    let cases = [
        (
            "/completion_manifest/status",
            json!({"completion_manifest":null}),
        ),
        ("/lane_registry/status", json!({"lane_registry":null})),
        ("/ready_for_merge/status", json!({"ready_for_merge":null})),
        (
            "/verification_backlog/status",
            json!({"verification_backlog":null}),
        ),
        ("/plugin_manifest/status", json!({"plugin_manifest":null})),
        (
            "/automation_tick_receipt/status",
            json!({"automation_tick_receipt":null}),
        ),
        (
            "/validator_receipt/status",
            json!({"validator_receipt":null}),
        ),
        (
            "/ready_for_merge_receipts/0/status",
            json!({"ready_for_merge_receipts":[null]}),
        ),
        ("/amendments/0/status", json!({"amendments":[null]})),
    ];
    for (path, bad) in cases {
        let errors = crate::red::fixture::schema::errors(
            &store,
            &json!({"json_patch":[{"op":"replace","path":path}]}),
            &bad,
        );
        assert!(!errors.is_empty(), "{path}");
    }
}
