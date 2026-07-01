use serde_json::{Value, json};

use super::{GOLD_STACK_SOURCE, failures, repo_values};

fn source_row_index(doc: &Value, source_id: &str) -> usize {
    doc["sources"]
        .as_array()
        .expect("sources array")
        .iter()
        .position(|row| row.get("source_id").and_then(Value::as_str) == Some(source_id))
        .expect("source row")
}

#[test]
fn research_registry_rejects_stale_archive_metadata() {
    let (_root, cards, mut registry, trace) = repo_values();
    let row = source_row_index(&registry, GOLD_STACK_SOURCE);
    registry["sources"][row]["source_archive_path"] = json!("/tmp/wrong-archive.zip");
    registry["sources"][row]["source_archive_digest"] =
        json!(crate::self_tests::boundaries::support::sha('1'));
    registry["sources"][row]["source_archive_entries"] = json!([{
        "entry_path": "wrong-entry.md",
        "entry_digest": crate::self_tests::boundaries::support::sha('2'),
    }]);
    let failures = failures(&cards, &registry, &trace);
    for expected in [
        "research_registry_source_archive_path_stale:gold-standard-stack-developer-experience-governance-2026-07-01",
        "research_registry_source_archive_digest_stale:gold-standard-stack-developer-experience-governance-2026-07-01",
        "research_registry_source_archive_entries_stale:gold-standard-stack-developer-experience-governance-2026-07-01",
    ] {
        assert!(
            failures.iter().any(|item| item == expected),
            "{expected}\n{failures:#?}"
        );
    }
}

#[test]
fn research_card_rejects_missing_mandatory_archive_metadata() {
    let (_root, mut cards, mut registry, trace) = repo_values();
    let card_row = source_row_index(&cards, GOLD_STACK_SOURCE);
    let registry_row = source_row_index(&registry, GOLD_STACK_SOURCE);
    for field in [
        "source_archive_path",
        "source_archive_digest",
        "source_archive_entries",
    ] {
        cards["sources"][card_row]
            .as_object_mut()
            .expect("card row")
            .remove(field);
        registry["sources"][registry_row]
            .as_object_mut()
            .expect("registry row")
            .remove(field);
    }
    let failures = failures(&cards, &registry, &trace);
    for expected in [
        "research_source_archive_path_missing:gold-standard-stack-developer-experience-governance-2026-07-01",
        "research_source_archive_digest_missing:gold-standard-stack-developer-experience-governance-2026-07-01",
        "research_source_archive_entries_missing:gold-standard-stack-developer-experience-governance-2026-07-01",
    ] {
        assert!(
            failures.iter().any(|item| item == expected),
            "{expected}\n{failures:#?}"
        );
    }
}
