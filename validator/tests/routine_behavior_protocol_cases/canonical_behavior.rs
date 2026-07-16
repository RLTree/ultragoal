use super::{digest, refusal_kind};
use ultragoal::routine_work::{
    RustSourceFrameInput, RustSourceSyntaxErrorKind, encode_rust_source_syntax_frame,
    evaluate_rust_source_syntax_frame, rust_source_syntax_observation_json,
};

#[test]
fn canonical_frame_binds_ordered_exact_sources_and_derives_pass_from_parsing() {
    let first = b"pub fn alpha() -> u8 { 1 }\n";
    let second = b"mod nested { pub const BETA: u8 = 2; }\n";
    let first_digest = digest(first);
    let second_digest = digest(second);
    let inputs = [
        RustSourceFrameInput::new("src/alpha.rs", &first_digest, first.len() as u64, first),
        RustSourceFrameInput::new("src/beta.rs", &second_digest, second.len() as u64, second),
    ];
    let frame = encode_rust_source_syntax_frame(&inputs).expect("canonical frame");

    let outcome = evaluate_rust_source_syntax_frame(&frame);
    let observation = outcome.observation().expect("syntax should pass");
    assert_eq!(outcome.exit_code(), 0);
    assert_eq!(observation.behavior(), "rust-source-syntax-v1");
    assert_eq!(observation.frame_sha256(), digest(&frame));
    assert_eq!(observation.source_count(), 2);
    assert_eq!(
        observation.total_source_bytes(),
        (first.len() + second.len()) as u64
    );
    assert!(observation.source_bindings_sha256().starts_with("sha256:"));
    let encoded: serde_json::Value =
        serde_json::from_slice(&rust_source_syntax_observation_json(observation)).unwrap();
    assert_eq!(encoded["behavior_id"], "rust-source-syntax-v1");
    assert_eq!(encoded["frame_sha256"], observation.frame_sha256());
    assert_eq!(
        encoded["source_bindings_sha256"],
        observation.source_bindings_sha256()
    );
}

#[test]
fn encoder_refuses_noncanonical_or_unbound_inputs() {
    let source = b"fn valid() {}\n";
    let source_digest = digest(source);
    let duplicate = [
        RustSourceFrameInput::new("src/a.rs", &source_digest, source.len() as u64, source),
        RustSourceFrameInput::new("src/a.rs", &source_digest, source.len() as u64, source),
    ];
    assert_eq!(
        encode_rust_source_syntax_frame(&duplicate)
            .expect_err("duplicate")
            .kind(),
        RustSourceSyntaxErrorKind::DuplicatePath
    );

    let unordered = [
        RustSourceFrameInput::new("src/b.rs", &source_digest, source.len() as u64, source),
        RustSourceFrameInput::new("src/a.rs", &source_digest, source.len() as u64, source),
    ];
    assert_eq!(
        encode_rust_source_syntax_frame(&unordered)
            .expect_err("unordered")
            .kind(),
        RustSourceSyntaxErrorKind::NonCanonicalOrder
    );

    let zero_digest = format!("sha256:{}", "0".repeat(64));
    let wrong_digest = [RustSourceFrameInput::new(
        "src/a.rs",
        &zero_digest,
        source.len() as u64,
        source,
    )];
    assert_eq!(
        encode_rust_source_syntax_frame(&wrong_digest)
            .expect_err("digest mismatch")
            .kind(),
        RustSourceSyntaxErrorKind::DigestMismatch
    );

    let wrong_length = [RustSourceFrameInput::new(
        "src/a.rs",
        &source_digest,
        source.len() as u64 + 1,
        source,
    )];
    assert_eq!(
        encode_rust_source_syntax_frame(&wrong_length)
            .expect_err("length mismatch")
            .kind(),
        RustSourceSyntaxErrorKind::LengthMismatch
    );

    assert_eq!(
        encode_rust_source_syntax_frame(&[])
            .expect_err("empty work cannot pass")
            .kind(),
        RustSourceSyntaxErrorKind::SourceCountOutOfBounds
    );
    assert_eq!(
        refusal_kind(evaluate_rust_source_syntax_frame(
            b"RoutineCommandReport-v1"
        )),
        RustSourceSyntaxErrorKind::UnsupportedProtocol
    );
}
