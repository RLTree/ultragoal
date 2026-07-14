use super::test_support::{Repository, tree};
use super::{execute_invocation, read_context};
use crate::cli::successor::{OutputMode, ParseOutcome, parse_args};
use crate::inventory::InventoryBuilder;
use crate::observability::{EventStore, SemanticEvent};
use crate::state::derive_adopted;
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};

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
        value["observability"]["read_failure"]["schema_version"],
        "LocalStoreReadFailure-v1"
    );
    assert_eq!(
        value["observability"]["read_failure"]["class"],
        "path-boundary"
    );
    assert_eq!(value["observability"]["read_failure"]["stage"], "open");
    assert_eq!(
        value["observability"]["explanation"]["classification"],
        "unavailable"
    );
    assert!(!text.contains("outside-diagnose-store"));
    assert_eq!(tree(&repository.root), before_tree);
    assert_eq!(repository.status(), before_status);
    fs::remove_dir_all(outside).unwrap();
}

#[cfg(unix)]
#[test]
fn bound_local_store_detects_in_place_mutation_and_leaf_replacement() {
    let repository = Repository::new("diagnose-store-revalidation");
    let context = read_context(&repository.root).unwrap();
    let path = super::observe::store_path(&repository.root);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let event_store = EventStore::for_context(&path, &context, "successor-runtime").unwrap();
    let event = SemanticEvent::for_context(
        &context,
        "successor-runtime",
        "revalidation-seed",
        1,
        1,
        "check.run",
        "fail",
    )
    .unwrap();
    assert!(event_store.append(&event).unwrap());

    let bound =
        super::local_store::LocalStore::open(&repository.root, &context, "successor-runtime")
            .unwrap();
    let original = fs::read(&path).unwrap();
    let mut file = OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(b"X").unwrap();
    file.sync_data().unwrap();
    let failure = bound.revalidate().unwrap_err();
    assert_eq!(failure.stage(), "revalidate");
    assert_eq!(failure.class(), "concurrent-change");

    fs::write(&path, &original).unwrap();
    let rebound =
        super::local_store::LocalStore::open(&repository.root, &context, "successor-runtime")
            .unwrap();
    let replaced = path.with_file_name("successor-events.replaced.jsonl");
    fs::rename(&path, &replaced).unwrap();
    fs::write(&path, &original).unwrap();
    let failure = rebound.revalidate().unwrap_err();
    assert_eq!(failure.stage(), "revalidate");
    assert_eq!(failure.class(), "concurrent-change");
    assert_eq!(fs::read(&replaced).unwrap(), original);
}
