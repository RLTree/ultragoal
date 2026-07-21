use super::*;
use crate::cli::successor_public::strict;

#[test]
fn inception_missing_input_is_actionable_and_zero_write() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator has repository parent");
    assert!(!root.join("PRODUCT_SUCCESS_BRIEF.json").exists());
    let before = strict::zero_write_guard::capture(root).expect("capture repository state");
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "inspect", "inception"]).expect("inception route parses")
    else {
        panic!("inception route invokes");
    };

    let streams = execute_invocation(root, invocation).render(OutputMode::Json);

    assert_eq!(streams.exit_code, 1);
    assert!(streams.stderr.is_empty());
    let payload: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(payload["schema_version"], "ProductInceptionInput-v1");
    assert_eq!(payload["status"], "missing");
    assert_eq!(payload["brief_path"], "PRODUCT_SUCCESS_BRIEF.json");
    assert!(
        payload["missing_fields"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
    );
    assert_eq!(
        strict::zero_write_guard::capture(root).expect("recapture repository state"),
        before
    );
}
