use super::fixture::{
    Binding, PRIVATE_EMAIL, PRIVATE_PATH, PRIVATE_TOKEN, Repository, SelectedFinding,
};
use super::snapshot::observe;
use serde_json::Value;
use std::net::TcpListener;
use std::path::Path;
use std::process::Output;

pub(crate) fn machine(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

pub(crate) fn combined_text(output: &Output) -> String {
    let mut bytes = output.stdout.clone();
    bytes.extend_from_slice(&output.stderr);
    String::from_utf8_lossy(&bytes).into_owned()
}

pub(crate) fn assert_private_absent(output: &Output, root: &Path) {
    let text = combined_text(output);
    for private in [
        PRIVATE_TOKEN,
        PRIVATE_PATH,
        PRIVATE_EMAIL,
        root.to_string_lossy().as_ref(),
    ] {
        assert!(!text.contains(private), "private output: {text}");
    }
}

pub(crate) fn assert_payload_zero_write(
    repository: &Repository,
    args: &[&str],
    environment: &[(&str, &str)],
    exits: &[i32],
    schema: &str,
) -> (Output, Value) {
    let before = observe(repository.root());
    let first = repository.run_with_env(args, environment);
    assert!(
        exits.contains(&first.status.code().unwrap_or(-1)),
        "{first:?}"
    );
    assert!(first.stderr.is_empty(), "{first:?}");
    assert_private_absent(&first, repository.root());
    let value = machine(&first.stdout);
    assert_eq!(value["schema_version"], schema);
    assert_eq!(observe(repository.root()), before, "hidden write: {args:?}");
    let second = repository.run_with_env(args, environment);
    assert_eq!(second.status.code(), first.status.code());
    assert_eq!(second.stdout, first.stdout);
    assert_eq!(second.stderr, first.stderr);
    assert_eq!(observe(repository.root()), before, "repeat write: {args:?}");
    (first, value)
}

pub(crate) fn assert_diagnostic_zero_write(
    repository: &Repository,
    args: &[&str],
    expected_exit: i32,
) -> (Output, Value) {
    let before = observe(repository.root());
    let output = repository.run(args);
    assert_eq!(output.status.code(), Some(expected_exit), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert_private_absent(&output, repository.root());
    let value = machine(&output.stderr);
    assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
    assert_eq!(observe(repository.root()), before);
    (output, value)
}

pub(crate) fn binding(repository: &Repository) -> Binding {
    let (_, value) = assert_payload_zero_write(
        repository,
        &["--json", "observe", "query"],
        &[],
        &[0],
        "ObservabilityQuery-v1",
    );
    Binding {
        context_id: value["context_id"].as_str().unwrap().to_owned(),
        candidate_id: value["candidate_id"].as_str().unwrap().to_owned(),
        source_id: value["source_id"].as_str().unwrap().to_owned(),
    }
}

pub(crate) fn selected_finding(repository: &Repository) -> SelectedFinding {
    let (_, value) = assert_payload_zero_write(
        repository,
        &["--json", "inspect", "findings"],
        &[],
        &[1],
        "ProductStateFindings-v1",
    );
    let row = value["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["code"] == "parallel_authority")
        .unwrap();
    SelectedFinding {
        finding_id: row["finding_id"].as_str().unwrap().to_owned(),
        repair_id: row["repair"]["repair_id"].as_str().unwrap().to_owned(),
    }
}

pub(crate) fn assert_no_connection(listener: &TcpListener) {
    listener.set_nonblocking(true).unwrap();
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
