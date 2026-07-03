use crate::audit::contract::Failure;
use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn errors(out: &[Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

#[test]
fn product_fitness_receipt_and_claim_digest_edges_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("pf-claim-digest");
    let receipt_path = root.join("receipts/pf.json");
    write_json(
        &receipt_path,
        &json!({
            "schema":"harness-ultragoal.product-fitness-receipt.v1",
            "claim":{"id":"PF"},
            "target_revision":{"value":crate::self_tests::boundaries::workspace_fixtures::sha('a')},
            "target_audience":{"name":"operators"},
            "job_to_be_done":{"job":"job"},
            "context_of_use":{"context":"context"},
            "desired_user_outcome":{"outcome":"outcome"},
            "business_or_mission_outcome":{"outcome":"mission"},
            "critical_journey":{"id":"journey-a"},
            "accessibility_gate":{"journey_binding":"journey-b"},
            "proof_surface":{"kind":"quality_in_use"},
            "claim_ceiling":"withheld",
            "producer_actor_id":"producer",
            "reviewer_actor_id":"reviewer",
            "receipt_digest":crate::self_tests::boundaries::workspace_fixtures::sha('b'),
            "substitution_rejections":[{"rejected_substitute":"install success","reason":""}]
        }),
    );
    let receipt = crate::json_boundary::read_json(&receipt_path).expect("pf receipt");
    let receipt_failures = crate::audit::product::fitness::receipt::failures(&root, &receipt);
    assert!(
        receipt_failures.contains(&"product_fitness_accessibility_unbound_to_journey".to_string())
    );
    assert!(
        receipt_failures.contains(&"product_fitness_install_substituted_for_success".to_string())
    );

    let claim = json!({
        "id":"PF",
        "title":"Product readiness quality in use",
        "claim_ceiling_effect":"included",
        "requires_product_cohesion":true,
        "evidence":[{
            "kind":"product::fitness::receipt",
            "surface":"product::fitness",
            "path":"receipts/pf.json",
            "digest":crate::self_tests::boundaries::workspace_fixtures::sha('0')
        }]
    });
    let mut out = Vec::new();
    crate::claim_semantics::product::fitness::check(&claim, &root, &mut out);
    assert!(errors(&out).contains(&"product_fitness_receipt_digest_mismatch"));
    std::fs::remove_dir_all(root).expect("cleanup pf claim digest");
}

#[test]
fn live_promotion_dogfood_patch_and_lane_edges_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("claim-boundary-edges");
    let proof = root.join("artifacts/live.json");
    write_json(&proof, &json!({"ok":true}));
    let digest = crate::digest::file(&proof).expect("live digest");
    let mut out = Vec::new();
    crate::claim_semantics::claim::evidence::live_e2e_check(
        &json!({"id":"LIVE","evidence":[{
            "kind":"live_beneficial_e2e",
            "surface":"ui_browser",
            "path":"artifacts/live.json",
            "digest":digest,
            "produced_by_command_id":"ok",
            "live_beneficial_task":{"real_input_path":"real/input.json","output_artifact_path":"mock/output.json"}
        }]}),
        &json!({"commands":[{"id":"ok","exit":0,"artifact_path":"artifacts/live.json","artifact_digest":digest}]}),
        &root,
        &mut out,
    );
    assert!(errors(&out).contains(&"live_beneficial_e2e_not_live"));

    let mut promotion = Vec::new();
    crate::claim_semantics::promotion_receipt::check(
        &json!({
            "id":"PROMO",
            "title":"Install button succeeded",
            "claim_ceiling_effect":"included",
            "evidence":[{"id":"bad","kind":"promotion_receipt","surface":"external","path":"../escape.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('c')}]
        }),
        &root,
        &mut promotion,
    );
    assert!(errors(&promotion).contains(&"promotion_receipt_invalid"));

    let mut dogfood = Vec::new();
    crate::claim_semantics::dogfood_receipt::check(
        &json!({
            "id":"DOG",
            "title":"Real multi-lane dogfood",
            "claim_ceiling_effect":"included",
            "evidence":[{"id":"bad","kind":"dogfood_receipt","surface":"root_integration","path":"../escape.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('d')}]
        }),
        &root,
        &mut dogfood,
    );
    assert!(errors(&dogfood).contains(&"dogfood_receipt_invalid"));

    let patched = crate::claim_semantics::apply_patch(
        &json!({"rows":[1, 2],"obj":{"a":1}}),
        &json!([
            {"op":"remove","path":"/obj/a"},
            {"op":"replace","path":"/rows/1","value":9}
        ]),
    )
    .expect("patch applies");
    assert_eq!(patched, json!({"rows":[1,9],"obj":{}}));
    assert!(
        crate::claim_semantics::lane::dependency::dependency_release_bad(
            &json!({"dependency_release":null}),
            &json!({}),
            &json!({})
        )
    );
    let mut overlap = Vec::new();
    let lanes = json!([
        {"owned_paths":["src"]},
        {"owned_paths":["src/module"]}
    ]);
    let lane_rows = lanes.as_array().expect("lane rows").clone();
    crate::claim_semantics::lane::root::scope::lane_overlap(&lane_rows, &mut overlap);
    assert!(errors(&overlap).contains(&"active_lane_owned_path_overlap"));
    std::fs::remove_dir_all(root).expect("cleanup claim boundary edges");
}

#[test]
fn plugin_and_product_classification_parsers_cover_quoted_values_and_product_class() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("plugin-product-parser");
    let agent = root.join("custom-agents/demo.toml");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    std::fs::create_dir_all(agent.parent().expect("agent parent")).expect("agent parent");
    std::fs::write(
        &agent,
        "name = \"Demo Agent\"\ndescription = \"Demo\"\ndeveloper_instructions = \"Do work\"\n",
    )
    .expect("agent toml");
    let mut out = Vec::new();
    crate::claim_semantics::plugin_policy::check_plugin(
        &json!({"plugin_manifest":{"resources":["custom-agents/demo.toml"],"skills":[],"agents":[{
            "name":"demo",
            "path":"custom-agents/demo.toml",
            "app_visible_name":"Demo Agent"
        }]}}),
        &root,
        &mut out,
    );
    assert!(
        !out.iter()
            .any(|failure| failure.error == "custom_agent_name_mismatch"),
        "{out:?}"
    );

    let product_failures =
        crate::claim_semantics::product::cohesion::product_cohesion_failures(&json!({
            "id":"PRODUCT",
            "title":"Internal product control",
            "claim_ceiling_effect":"withheld_or_blocked",
            "product_applicability":{"surface_classification":"control_surface"}
        }));
    assert!(product_failures.is_empty(), "{product_failures:?}");
    std::fs::remove_dir_all(root).expect("cleanup plugin product parser");
}
