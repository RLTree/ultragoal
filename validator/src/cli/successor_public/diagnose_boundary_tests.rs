use super::test_support::{Repository, tree};
use super::{execute_invocation, read_context};
use crate::cli::successor::{OutputMode, ParseOutcome, parse_args};
use crate::inventory::InventoryBuilder;
use crate::state::derive_adopted;
use std::fs;

#[cfg(unix)]
#[test]
fn substituted_store_is_unavailable_without_path_echo_or_hidden_writes() {
    let repository = Repository::new("diagnose-store-symlink");
    let context = read_context(&repository.root).unwrap();
    let inventory = InventoryBuilder::new(&context).build().unwrap();
    let state = derive_adopted(&context, &inventory).unwrap();
    let finding = state.findings().first().expect("fixture has findings");
    let outside = repository.root.with_extension("outside-diagnose-store");
    fs::create_dir_all(repository.root.join("validation_artifacts/observability")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(
        &outside,
        repository
            .root
            .join("validation_artifacts/observability/spool"),
    )
    .unwrap();
    let before_tree = tree(&repository.root);
    let before_status = repository.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "diagnose", "--finding", &finding.finding_id]).unwrap()
    else {
        panic!("expected invocation")
    };

    let streams = execute_invocation(&repository.root, invocation).render(OutputMode::Json);

    assert_eq!(streams.exit_code, 1);
    assert!(streams.stderr.is_empty());
    let text = String::from_utf8(streams.stdout).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["observability"]["store_status"], "unavailable");
    assert_eq!(
        value["observability"]["explanation"]["classification"],
        "unavailable"
    );
    assert!(!text.contains("outside-diagnose-store"));
    assert_eq!(tree(&repository.root), before_tree);
    assert_eq!(repository.status(), before_status);
    fs::remove_dir_all(outside).unwrap();
}
