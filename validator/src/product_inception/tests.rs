use super::model::{BriefV2, CandidateBinding, ContractFacts};
use super::normalize;
use super::parser::{ParsedBrief, parse};
use super::reader::{confined, regular_file};
use super::validation::validate_v2;
use crate::context::EffectClass;
use crate::state::{ActionDefinition, ActionKind, AuthorityRequirement};
use serde_json::json;
use std::fs;
use std::path::Path;

fn digest(ch: char) -> String {
    format!("sha256:{}", ch.to_string().repeat(64))
}

fn facts() -> ContractFacts {
    ContractFacts {
        product_contract_id: "PSC-1".to_owned(),
        contract_version: "2.3.0".to_owned(),
        contract_digest: digest('a'),
        authority_contract_id: "harness-ultragoal-successor-contract-v2".to_owned(),
        claim_registry_digest: digest('b'),
        claim_ids: vec!["CL-SOURCE".to_owned()],
        public_surface_catalog_digest: digest('c'),
        surface_ids: vec!["PS-ENTRY".to_owned(), "PS-CLI".to_owned()],
    }
}

fn candidate() -> CandidateBinding {
    CandidateBinding {
        head_commit: Some("commit".to_owned()),
        head_tree: Some("tree".to_owned()),
        branch: Some("branch".to_owned()),
        dirty: true,
        subject_dirty: true,
        candidate_digest: digest('d'),
        repository_digest: digest('e'),
    }
}

fn brief() -> BriefV2 {
    serde_json::from_value(json!({
        "schema": "harness-ultragoal.product-success-brief.v2",
        "product_success_contract_id": "PSC-1",
        "product_success_contract_digest": digest('a'),
        "claim_ids": ["CL-SOURCE"],
        "target_problem": "problem",
        "audience": "operator",
        "job_to_be_done": "job",
        "context_of_use": "repository",
        "desired_outcome": "outcome",
        "first_value_event": "first value",
        "operator": {"kind": "agent", "actor_reference": "operator-1"},
        "real_work": {
            "repository_identity": digest('e'),
            "starting_candidate": digest('d'),
            "dirty_state_expectation": "dirty",
            "task_id": "task-1",
            "task": "task",
            "expected_useful_outcome": "useful outcome"
        },
        "public_entry_surface": {
            "surface_id": "PS-ENTRY",
            "route": "harness-ultragoal",
            "forbidden_bypasses": ["direct internal command"]
        },
        "protected_invariants": [{
            "id": "preserve-state",
            "claim_ids": ["CL-SOURCE"],
            "surfaces": ["PS-ENTRY"],
            "required_condition": "unrelated bytes remain unchanged",
            "disposition": "fail_closed"
        }],
        "first_truth_loop": {
            "loop_id": "loop-1",
            "positive_path": [{
                "transition_id": "inspect",
                "action_id": "inspect-product-inception",
                "command_id": "inspect-inception",
                "effect": "read",
                "order": 1,
                "dependency_ids": [],
                "capability_ids": ["read"],
                "claim_ids": ["CL-SOURCE"],
                "product_surfaces": ["PS-ENTRY"],
                "expected_observation": "read without writes",
                "evidence_class": "source"
            }],
            "first_value_transition": "inspect",
            "failure_control": {
                "transition_id": "inspect",
                "failure": "candidate changes",
                "diagnosis": "revalidate",
                "recovery": "retry",
                "preservation": "no writes"
            },
            "preservation_expectation": "unrelated bytes remain",
            "repeat_use_expectation": "repeatable"
        },
        "depth_triggers": [],
        "evidence_class": "source",
        "evidence_ladder": "source",
        "claim_ceiling": "source_only"
    }))
    .expect("valid v2 brief")
}

#[test]
fn parser_accepts_v2_and_rejects_unknown_fields() {
    let value = serde_json::to_vec(&brief()).unwrap();
    assert!(matches!(parse(&value), Ok(ParsedBrief::EvidenceLed(_))));

    let mut object: serde_json::Value = serde_json::from_slice(&value).unwrap();
    object["unexpected"] = json!(true);
    assert!(matches!(
        parse(&serde_json::to_vec(&object).unwrap()),
        Err("brief_v2_shape_invalid")
    ));
}

#[test]
fn semantic_validation_rejects_repository_and_claim_substitution() {
    let value = brief();
    let mut wrong_repository = candidate();
    wrong_repository.repository_digest = digest('f');
    assert_eq!(
        validate_v2(&value, &facts(), &wrong_repository),
        Err("brief_candidate_binding_stale")
    );

    let mut value = brief();
    value.claim_ids = vec!["CL-UNKNOWN".to_owned()];
    assert_eq!(
        validate_v2(&value, &facts(), &candidate()),
        Err("brief_claim_unknown")
    );
}

#[test]
fn historical_starting_candidate_does_not_stale_an_expected_dirty_journey() {
    let mut value = brief();
    value.real_work.dirty_state_expectation = super::model::DirtyStateExpectation::Either;
    let mut current = candidate();
    current.candidate_digest = digest('f');
    assert!(validate_v2(&value, &facts(), &current).is_ok());
}

#[test]
fn product_success_contract_facts_require_version_2_3_0() {
    assert!(super::reader::product_contract_version_supported("2.3.0"));
    assert!(!super::reader::product_contract_version_supported("2.2.0"));
}

#[test]
fn semantic_validation_rejects_noncontiguous_truth_loop() {
    let mut value = brief();
    value.first_truth_loop.positive_path[0].order = 2;
    assert_eq!(
        validate_v2(&value, &facts(), &candidate()),
        Err("brief_truth_loop_invalid")
    );
}

#[test]
fn semantic_validation_rejects_claim_ceiling_above_evidence_class() {
    let mut value = brief();
    value.claim_ceiling = super::model::InceptionClaimCeiling::InstalledOnly;
    assert_eq!(
        validate_v2(&value, &facts(), &candidate()),
        Err("brief_claim_ceiling_exceeds_evidence")
    );
}

#[test]
fn validated_transition_binds_only_the_exact_root_action() {
    let mut actions = vec![ActionDefinition {
        action_id: "inspect-product-inception".to_owned(),
        priority: 50,
        kind: ActionKind::Command,
        repair_id: "repair-product-inception".to_owned(),
        requires_dependencies: Vec::new(),
        required_capabilities: vec!["read".to_owned()],
        effect: EffectClass::Read,
        authority: AuthorityRequirement::Root,
        command_id: Some("inspect-inception".to_owned()),
        authority_request: None,
        evidence_led: None,
    }];
    assert!(matches!(
        super::ranking::bind_validated(&brief(), &digest('f'), &mut actions, &Default::default(),),
        Ok(super::ranking::RankingDisposition::Active)
    ));
    let binding = actions[0]
        .evidence_led
        .as_ref()
        .expect("root binding issued");
    assert_eq!(binding.transition_id.as_deref(), Some("inspect"));
    assert_eq!(binding.transition_order, Some(1));

    let mut wrong = brief();
    wrong.first_truth_loop.positive_path[0].command_id = "wrong-command".to_owned();
    assert!(matches!(
        super::ranking::bind_validated(&wrong, &digest('f'), &mut actions, &Default::default(),),
        Ok(super::ranking::RankingDisposition::InceptionRequired)
    ));
}

#[test]
fn normalization_canonicalizes_unordered_semantic_lists_without_deduplication() {
    let mut value = brief();
    value.claim_ids = vec!["CL-SOURCE".to_owned()];
    value.public_entry_surface.forbidden_bypasses = vec!["z".to_owned(), "a".to_owned()];
    value.protected_invariants[0].claim_ids = vec!["CL-SOURCE".to_owned()];
    value.protected_invariants[0].surfaces = vec!["PS-ENTRY".to_owned()];
    normalize::v2(&mut value);
    assert_eq!(value.public_entry_surface.forbidden_bypasses, ["a", "z"]);
    assert_eq!(value.claim_ids, ["CL-SOURCE"]);
}

#[test]
fn confined_reader_rejects_escape_symlink_and_hard_linked_input() {
    let root = std::env::temp_dir().join(format!("ultragoal-inception-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let file = root.join("brief.json");
    fs::write(&file, b"{}").unwrap();
    assert!(confined(&root, "../brief.json").is_err());
    regular_file(&file).unwrap();
    assert!(regular_file(&root).is_err());

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&file, root.join("symlink.json")).unwrap();
        assert!(regular_file(&root.join("symlink.json")).is_err());
        fs::hard_link(&file, root.join("hard.json")).unwrap();
        assert!(regular_file(&file).is_err());
    }
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn projection_inputs_never_include_private_root_text() {
    let value = serde_json::to_string(&brief()).unwrap();
    assert!(!value.contains(Path::new("/private").to_string_lossy().as_ref()));
}
