use super::*;
use crate::cli::successor_public::strict;

#[test]
fn inception_projects_current_authority_without_writes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator has repository parent");
    let before = strict::zero_write_guard::capture(root).expect("capture repository state");
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "inspect", "inception"]).expect("inception route parses")
    else {
        panic!("inception route invokes");
    };

    let streams = execute_invocation(root, invocation).render(OutputMode::Json);

    assert_eq!(streams.exit_code, 0);
    assert!(streams.stderr.is_empty());
    let payload: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(payload["schema_version"], "ProductInception-v2");
    assert_eq!(payload["status"], "current");
    assert_eq!(payload["claim_ids"], serde_json::json!(["CL-USABLE-LOOP"]));
    assert_eq!(payload["ranking_eligible"], false);
    assert!(
        payload["claim_ceiling"]
            .as_str()
            .is_some_and(|ceiling| ceiling.contains("remains withheld"))
    );
    assert!(payload.get("brief_path").is_none());
    assert!(payload.get("brief_digest").is_none());
    assert_eq!(
        strict::zero_write_guard::capture(root).expect("recapture repository state"),
        before
    );
}
