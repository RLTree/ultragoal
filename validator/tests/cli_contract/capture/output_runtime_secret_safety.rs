#[cfg(target_os = "macos")]
#[test]
fn true_eof_secret_prefix_is_absent_from_the_canonical_run() {
    use super::capture::{CommandSpec, PublicArg, SecretArg};
    use super::fixture::RepoFixture;
    use crate::context::{BuildRequest, LiveContext};
    use sha2::{Digest, Sha256};

    let fixture = RepoFixture::new("true-eof-secret-prefix");
    let secret = "zzq7V5-canonical-true-eof-secret";
    let cut = 7;
    let context = LiveContext::build(
        BuildRequest::new(fixture.root())
            .expect_repository_root(fixture.root())
            .expect_worktree_root(fixture.root())
            .bind_secret_source("CAPTURE_SECRET", "v1")
            .probe_tool("sandbox-exec")
            .probe_tool("printf"),
    )
    .unwrap();
    let run = CommandSpec::catalog_read("true-eof-secret-prefix", "printf")
        .public_arg(PublicArg::new(format!("%.{cut}s")))
        .secret_arg(SecretArg::new(secret, "CAPTURE_SECRET"))
        .run(&context)
        .unwrap();

    assert!(run.stdout_bytes().is_empty());
    assert!(run.stderr_bytes().is_empty());
    let json = run.to_canonical_json().unwrap();
    let text = String::from_utf8_lossy(&json);
    let debug = format!("{run:?}");
    for bytes in [secret.as_bytes(), &secret.as_bytes()[..cut]] {
        assert!(!json.windows(bytes.len()).any(|seen| seen == bytes));
        let digest = format!("{:x}", Sha256::digest(bytes));
        assert!(!text.contains(&digest));
        assert!(!debug.contains(String::from_utf8_lossy(bytes).as_ref()));
        assert!(!debug.contains(&digest));
    }
    let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(
        value["termination"]["kind"],
        "withheld-secret-bearing-invocation"
    );
    assert_eq!(run.monotonic_duration_ns(), 0);
    assert!(!run.interrupted());
    assert_eq!(
        value["stdout"]["content_disposition"],
        "withheld-secret-bearing-invocation"
    );
    assert_eq!(value["stdout"]["truncated"], true);
    assert_eq!(value["stdout"]["observation_limit_exceeded"], false);
    assert_eq!(value["stdout"]["captured_byte_length"], 0);
    assert_eq!(
        value["stdout"]["captured_sha256"],
        format!("sha256:{:x}", Sha256::digest([]))
    );
}
