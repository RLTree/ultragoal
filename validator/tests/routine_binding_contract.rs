use sha2::{Digest, Sha256};
use ultragoal::routine_work::{
    RustSourceFrameInput, RustSourceSyntaxErrorKind, RustSourceSyntaxOutcome,
    encode_rust_source_syntax_frame, evaluate_rust_source_syntax_frame,
    rust_source_syntax_observation_json,
};

#[test]
fn canonical_frame_is_the_only_public_routine_behavior_input() {
    let source = b"pub fn value() -> u8 { 9 }\n";
    let digest = format!("sha256:{:x}", Sha256::digest(source));
    let frame = encode_rust_source_syntax_frame(&[RustSourceFrameInput::new(
        "src/lib.rs",
        &digest,
        source.len() as u64,
        source,
    )])
    .unwrap();

    let RustSourceSyntaxOutcome::Passed(observation) = evaluate_rust_source_syntax_frame(&frame)
    else {
        panic!("canonical typed frame refused")
    };
    assert_eq!(observation.behavior(), "rust-source-syntax-v1");
    assert_eq!(observation.source_count(), 1);
    assert_eq!(observation.total_source_bytes(), source.len() as u64);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&rust_source_syntax_observation_json(
            &observation
        ))
        .unwrap()["schema_version"],
        "RoutineBehaviorObservation-v1"
    );
}

#[test]
fn arbitrary_unframed_bytes_refuse_without_an_observation() {
    let outcome = evaluate_rust_source_syntax_frame(b"pub fn attacker() {}\n");
    assert_eq!(outcome.exit_code(), 2);
    assert!(outcome.observation().is_none());
    assert_eq!(
        outcome.refusal().unwrap().kind(),
        RustSourceSyntaxErrorKind::UnsupportedProtocol
    );
}
