use serde_json::{Value, json};

fn row(id: &str, status: &str, gate: &str, refs: Value, claims: &str) -> Value {
    json!({
        "id": id,
        "source_law": "law",
        "required_behavior": "behavior",
        "enforcement_status": status,
        "gate_or_fixture_path": gate,
        "owner_lane": "lane",
        "claim_ids_affected": claims,
        "current_status": "current",
        "blocker_or_repair_action": "repair",
        "required_execplan_refs": refs
    })
}

fn contains(items: &[String], needle: &str) -> bool {
    items.iter().any(|item| item.contains(needle))
}

fn write_text(path: &std::path::Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, text).expect("write text");
}

#[test]
fn agent_standards_value_failures_cover_status_and_gate_branches() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("agent-standards-branches");
    std::fs::create_dir_all(root.join("templates/scripts")).expect("scripts");
    std::fs::write(
        root.join("templates/scripts/check-agent-standards"),
        "#!/bin/sh\n",
    )
    .expect("script");
    std::fs::create_dir_all(root.join("fixtures/red")).expect("fixture dir");
    std::fs::write(root.join("fixtures/red/x.json"), "{}").expect("direct gate");

    let failures = crate::audit::agent::standards::enforcement::value_failures(&json!({}), None);
    assert_eq!(failures, ["agent_standards_rows_missing"]);

    let missing = json!({
        "id": "",
        "enforcement_status": "mechanized",
        "source_law": "",
        "required_behavior": "",
        "owner_lane": "",
        "current_status": "",
        "blocker_or_repair_action": "",
        "required_execplan_refs": [],
        "gate_or_fixture_path": ""
    });
    let rows = json!({"rows": [
        missing,
        row("dup-row", "mechanized", "missing/gate.json", json!(["E1"]), "CLAIM"),
        row("dup-row", "backlogged", "fixtures/red/x.json", json!(["E1"]), "CLAIM"),
        row("blocked-row", "blocked", "fixtures/red/x.json", json!(["E1"]), "CLAIM"),
        row("info-row", "informational", "", json!(["E1"]), "CLAIM"),
        row("unknown-row", "unknown", "", json!(["E1"]), "none"),
        row("invalid-row", "unexpected", "", json!(["E1"]), "none"),
        row("script-row", "mechanized", "scripts/check-agent-standards", json!(["E1"]), "CLAIM"),
        row("direct-row", "mechanized", "fixtures/red/x.json", json!(["E1"]), "CLAIM"),
        row("escape-row", "mechanized", "../escape.json", json!(["E1"]), "CLAIM")
    ]});
    let failures = crate::audit::agent::standards::enforcement::value_failures(&rows, Some(&root));
    for expected in [
        "agent_standards_row_missing_id",
        "agent_standards_row_missing_source_law:",
        "agent_standards_row_missing_required_behavior:",
        "agent_standards_row_missing_owner_lane:",
        "agent_standards_row_missing_current_status:",
        "agent_standards_debt_without_action:",
        "agent_standards_execplan_refs_missing:",
        "agent_standards_mechanized_without_gate:",
        "agent_standards_duplicate_row:dup-row",
        "agent_standards_gate_missing:dup-row:missing/gate.json",
        "agent_standards_unmechanized_material_claim:dup-row:backlogged",
        "agent_standards_unmechanized_material_claim:blocked-row:blocked",
        "agent_standards_informational_overclaims:info-row",
        "agent_standards_row_unclassified:unknown-row",
        "agent_standards_status_invalid:invalid-row",
    ] {
        assert!(contains(&failures, expected), "{expected}: {failures:?}");
    }
    assert!(!contains(
        &failures,
        "agent_standards_gate_missing:script-row"
    ));
    assert!(!contains(
        &failures,
        "agent_standards_gate_missing:direct-row"
    ));
    assert!(contains(
        &failures,
        "agent_standards_gate_missing:escape-row:../escape.json"
    ));
    let rootless = json!({"rows": [
        row("rootless", "mechanized", "fixtures/red/x.json", json!(["E1"]), "CLAIM")
    ]});
    let rootless_failures =
        crate::audit::agent::standards::enforcement::value_failures(&rootless, None);
    assert!(
        !contains(&rootless_failures, "agent_standards_gate_missing:rootless"),
        "{rootless_failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn agent_standards_tsv_checks_cover_parse_and_script_evidence_paths() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "agent-standards-tsv-branches",
    );
    let failures = crate::audit::agent::standards::tsv::checks::failures(
        &root,
        "missing.tsv",
        "audit.tsv",
        &json!({"rows":[]}),
    );
    assert!(contains(&failures, "agent_standards_tsv_read_failed"));

    write_text(
        &root.join("standards.tsv"),
        "id\tsource_law\trequired_behavior\tenforcement_status\tgate_or_fixture_path\towner_lane\tclaim_ids_affected\tcurrent_status\tblocker_or_repair_action\trequired_execplan_refs\nrow1\tlaw\tbehavior\tdeterministic_fail_closed\tfixtures/red/x.json\towner\tclaim\tcurrent\trepair\tE1\n",
    );
    let failures = crate::audit::agent::standards::tsv::checks::failures(
        &root,
        "standards.tsv",
        "missing-audit.tsv",
        &json!({"rows":[]}),
    );
    assert!(contains(&failures, "agent_standards_tsv_read_failed"));

    write_text(&root.join("standards.tsv"), "bad\theader\nrow\tvalue\n");
    let failures = crate::audit::agent::standards::tsv::checks::failures(
        &root,
        "standards.tsv",
        "audit.tsv",
        &json!({"rows":[]}),
    );
    assert!(contains(&failures, "agent_standards_tsv_bad_header"));

    write_text(
        &root.join("standards.tsv"),
        "id\tsource_law\trequired_behavior\tenforcement_status\tgate_or_fixture_path\towner_lane\tclaim_ids_affected\tcurrent_status\tblocker_or_repair_action\trequired_execplan_refs\nrow1\tlaw\tbehavior\tdeterministic_fail_closed\tscripts/check-agent-standards\towner\tclaim\tcurrent\trepair\tE1\n",
    );
    write_text(
        &root.join("templates/scripts/check-agent-standards"),
        "#!/bin/sh\n",
    );
    let digest =
        crate::digest::file(&root.join("templates/scripts/check-agent-standards")).expect("digest");
    write_text(
        &root.join("audit.tsv"),
        &format!(
            "standard_id\taudit_status\tevidence_path\tevidence_digest\taudited_at\tclaim_ceiling_impact\nrow1\tpass\tscripts/check-agent-standards\t{digest}\t2026-06-25T00:00:00Z\tblocks\n"
        ),
    );
    let failures = crate::audit::agent::standards::tsv::checks::failures(
        &root,
        "standards.tsv",
        "audit.tsv",
        &json!({"rows":[{"id":"row1","enforcement_status":"deterministic_fail_closed","required_execplan_refs":["E1"]}]}),
    );
    assert!(failures.is_empty(), "{failures:?}");

    assert_eq!(
        crate::audit::agent::standards::tsv::checks::audit_row_failures(&root, &json!("bad")),
        ["agent_standards_audit_evidence_invalid"]
    );
    assert!(
        crate::audit::agent::standards::tsv::checks::audit_row_failures(
            &root,
            &json!({"standard_id":"row1","audit_status":"fail"})
        )
        .is_empty()
    );
    std::fs::remove_dir_all(root).expect("cleanup agent standards tsv branches");
}

#[test]
fn agent_standards_tsv_checks_reject_bad_rows_and_stale_audits() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "agent-standards-tsv-failures",
    );
    write_text(
        &root.join("standards.tsv"),
        "id\tsource_law\trequired_behavior\tenforcement_status\tgate_or_fixture_path\towner_lane\tclaim_ids_affected\tcurrent_status\tblocker_or_repair_action\trequired_execplan_refs\nshort\trow\n",
    );
    let failures = crate::audit::agent::standards::tsv::checks::failures(
        &root,
        "standards.tsv",
        "audit.tsv",
        &json!({"rows":[]}),
    );
    assert!(contains(&failures, "agent_standards_tsv_bad_row"));

    write_text(
        &root.join("standards.tsv"),
        "id\tsource_law\trequired_behavior\tenforcement_status\tgate_or_fixture_path\towner_lane\tclaim_ids_affected\tcurrent_status\tblocker_or_repair_action\trequired_execplan_refs\nrow1\tlaw\tbehavior\tdeterministic_fail_closed\tfixtures/red/x.json\towner\tclaim\tcurrent\trepair\tE1\nrow2\tlaw\tbehavior\tdeterministic_fail_closed\tfixtures/red/y.json\towner\tclaim\tcurrent\trepair\tE2\n",
    );
    write_text(
        &root.join("audit.tsv"),
        "standard_id\taudit_status\tevidence_path\tevidence_digest\taudited_at\tclaim_ceiling_impact\nrow1\tpass\t\tsha256:1111111111111111111111111111111111111111111111111111111111111111\t2026-06-25T00:00:00Z\tblocks\nrow3\tfail\tmissing.json\tsha256:1111111111111111111111111111111111111111111111111111111111111111\t2026-06-25T00:00:00Z\tblocks\n",
    );
    let failures = crate::audit::agent::standards::tsv::checks::failures(
        &root,
        "standards.tsv",
        "audit.tsv",
        &json!({"rows":[
            {"id":"row1","enforcement_status":"deterministic_fail_closed","required_execplan_refs":["E1"]},
            {"id":"row2","enforcement_status":"deterministic_fail_closed","required_execplan_refs":["E2"]}
        ]}),
    );
    for expected in [
        "agent_standards_audit_missing:row2",
        "agent_standards_audit_evidence_invalid:row1",
        "agent_standards_audit_not_pass:row3:fail",
    ] {
        assert!(contains(&failures, expected), "{expected}: {failures:?}");
    }
    std::fs::remove_dir_all(root).expect("cleanup stale audits");
}
