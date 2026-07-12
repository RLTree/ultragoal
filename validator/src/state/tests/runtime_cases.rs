use super::fixture::*;
use crate::context::EffectClass;
use crate::state::catalog::{RuntimeField, RuntimeRequirement, RuntimeSource, RuntimeValue};
use crate::state::engine::derive_bound;
use crate::state::types::AuthorityRequirement;
use std::collections::BTreeSet;
use std::fs;
use std::process::Command;

fn model_requirement() -> RuntimeRequirement {
    RuntimeRequirement {
        field: RuntimeField::ModelFamily,
        scope: scope("runtime"),
        repair: repair(
            "expose-model-family",
            AuthorityRequirement::Root,
            EffectClass::Read,
        ),
        ceiling_reductions: reduction(&["runtime"]),
    }
}

#[test]
fn exposed_runtime_metadata_is_recorded_verbatim_with_its_source() {
    let mut spec = spec();
    spec.runtime_metadata.model_family = Some(RuntimeValue::test_exposed(
        "GPT-5",
        RuntimeSource::CodexExposed,
        "codex-session-metadata:model-family",
        "sha256:context",
    ));
    spec.runtime_requirements.push(model_requirement());
    let state = derive_bound(inputs(), &catalog(spec)).unwrap();
    assert!(state.findings().is_empty());
    let observed = state.runtime_metadata().model_family.as_ref().unwrap();
    assert_eq!(observed.value(), "GPT-5");
    assert_eq!(
        observed.exposed_source(),
        "codex-session-metadata:model-family"
    );
    assert!(state.claim_ceilings()[0].dimensions().contains("runtime"));
}

#[test]
fn a_value_without_exposed_source_provenance_fails_closed() {
    let mut spec = spec();
    spec.runtime_metadata.model_family = Some(RuntimeValue::test_exposed(
        "requested-in-prompt",
        RuntimeSource::CodexExposed,
        "",
        "sha256:context",
    ));
    spec.runtime_requirements.push(model_requirement());
    let error =
        crate::state::catalog::DependencyActionCatalog::from_untrusted_spec(spec).unwrap_err();
    assert!(error.to_string().contains("invalid-runtime-provenance"));
}

#[test]
fn product_state_is_the_runtime_authority_for_summary_diagnose_and_next() {
    let root = std::env::temp_dir().join(format!("ultragoal-state-runtime-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q"]);
    git(
        &root,
        &["config", "user.email", "state-runtime@example.invalid"],
    );
    git(&root, &["config", "user.name", "State Runtime"]);
    fs::write(root.join("tracked"), b"tracked\n").unwrap();
    git(&root, &["add", "tracked"]);
    git(&root, &["commit", "-qm", "fixture"]);
    let context =
        crate::context::LiveContext::build(crate::context::BuildRequest::new(&root)).unwrap();
    let mut bound = inputs();
    bound.context_id = context.context_id().to_owned();
    bound.authority_catalog_context_id = context.context_id().to_owned();
    let mut state_spec = spec();
    state_spec.expected_context_id = context.context_id().to_owned();
    let catalog = catalog_for_binding(
        state_spec,
        &bound.context_id,
        &bound.authority_catalog_id,
        CANDIDATE_ID,
        BTreeSet::new(),
    )
    .unwrap();
    let state = derive_bound(bound, &catalog).unwrap();
    let before = status(&root);

    for (argv, expected) in [
        (vec!["--json", "inspect"], state.summary_json().unwrap()),
        (
            vec!["--json", "inspect", "findings"],
            state.findings_json().unwrap(),
        ),
        (
            vec!["--json", "inspect", "claims"],
            state.claims_json().unwrap(),
        ),
        (vec!["--json", "diagnose"], state.diagnose_json().unwrap()),
        (vec!["--json", "next"], state.next_json().unwrap()),
    ] {
        let crate::cli::successor::ParseOutcome::Invocation(invocation) =
            crate::cli::successor::parse_args(argv).unwrap()
        else {
            panic!("expected invocation");
        };
        let streams = crate::cli::successor::runtime::RuntimeSession::new(&context, Some(&state))
            .dispatch(&invocation)
            .render(crate::cli::successor::OutputMode::Json);
        assert_eq!(streams.exit_code, 0);
        assert!(streams.stderr.is_empty());
        assert_eq!(
            streams.stdout.strip_suffix(b"\n"),
            Some(expected.as_slice())
        );
    }
    assert_eq!(status(&root), before);
    fs::remove_dir_all(root).unwrap();
}

fn git(root: &std::path::Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
}

fn status(root: &std::path::Path) -> Vec<u8> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "-z", "--untracked-files=all"])
        .env("GIT_OPTIONAL_LOCKS", "0")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    output.stdout
}
