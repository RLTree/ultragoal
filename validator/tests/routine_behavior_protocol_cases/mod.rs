mod canonical_behavior;
mod refusal_matrix;

use sha2::{Digest, Sha256};
use ultragoal::routine_work::{RustSourceSyntaxErrorKind, RustSourceSyntaxOutcome};

const PREFIX: &[u8] = b"HUL-RSS-FRAME\0";
const SCHEMA: &[u8] = b"rust-source-syntax-v1";

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn raw_frame(rows: &[(&[u8], &[u8], Option<u64>)]) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.extend_from_slice(PREFIX);
    frame.extend_from_slice(&(SCHEMA.len() as u16).to_be_bytes());
    frame.extend_from_slice(SCHEMA);
    frame.extend_from_slice(&(rows.len() as u32).to_be_bytes());
    for (path, source, declared_length) in rows {
        frame.extend_from_slice(&(path.len() as u16).to_be_bytes());
        frame.extend_from_slice(path);
        frame.extend_from_slice(&Sha256::digest(source));
        frame.extend_from_slice(&declared_length.unwrap_or(source.len() as u64).to_be_bytes());
        frame.extend_from_slice(source);
    }
    frame
}

fn refusal_kind(outcome: RustSourceSyntaxOutcome) -> RustSourceSyntaxErrorKind {
    let error = outcome.refusal().expect("expected typed refusal");
    assert_eq!(outcome.exit_code(), 2);
    error.kind()
}
