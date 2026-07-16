use sha2::{Digest, Sha256};

use super::rust_source_frame::parse_frame;

pub const RUST_SOURCE_SYNTAX_BEHAVIOR: &str = "rust-source-syntax-v1";
pub const RUST_SOURCE_SYNTAX_REFUSAL_EXIT: i32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustSourceSyntaxErrorKind {
    UnsupportedProtocol,
    FrameTooLarge,
    SourceCountOutOfBounds,
    TruncatedFrame,
    TrailingBytes,
    InvalidPathUtf8,
    InvalidPath,
    DuplicatePath,
    NonCanonicalOrder,
    InvalidDigest,
    DigestMismatch,
    SourceTooLarge,
    LengthMismatch,
    InvalidSourceUtf8,
    InvalidRustSyntax,
    IntegerOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustSourceSyntaxError {
    kind: RustSourceSyntaxErrorKind,
}

impl RustSourceSyntaxError {
    pub fn kind(&self) -> RustSourceSyntaxErrorKind {
        self.kind
    }

    pub fn exit_code(&self) -> i32 {
        RUST_SOURCE_SYNTAX_REFUSAL_EXIT
    }
}

impl std::fmt::Display for RustSourceSyntaxError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "rust-source-syntax-refused:{:?}", self.kind)
    }
}

impl std::error::Error for RustSourceSyntaxError {}

pub(super) fn error(kind: RustSourceSyntaxErrorKind) -> RustSourceSyntaxError {
    RustSourceSyntaxError { kind }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustSourceSyntaxObservation {
    frame_sha256: String,
    source_bindings_sha256: String,
    source_count: usize,
    total_source_bytes: u64,
}

impl RustSourceSyntaxObservation {
    pub fn behavior(&self) -> &'static str {
        RUST_SOURCE_SYNTAX_BEHAVIOR
    }

    pub fn frame_sha256(&self) -> &str {
        &self.frame_sha256
    }

    pub fn source_bindings_sha256(&self) -> &str {
        &self.source_bindings_sha256
    }

    pub fn source_count(&self) -> usize {
        self.source_count
    }

    pub fn total_source_bytes(&self) -> u64 {
        self.total_source_bytes
    }
}

pub fn rust_source_syntax_observation_json(observation: &RustSourceSyntaxObservation) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "schema_version": "RoutineBehaviorObservation-v1",
        "behavior_id": observation.behavior(),
        "frame_sha256": observation.frame_sha256(),
        "source_bindings_sha256": observation.source_bindings_sha256(),
        "source_count": observation.source_count(),
        "total_source_bytes": observation.total_source_bytes(),
    }))
    .expect("fixed routine behavior observation serializes")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RustSourceSyntaxOutcome {
    Passed(RustSourceSyntaxObservation),
    Refused(RustSourceSyntaxError),
}

impl RustSourceSyntaxOutcome {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Passed(_) => 0,
            Self::Refused(error) => error.exit_code(),
        }
    }

    pub fn observation(&self) -> Option<&RustSourceSyntaxObservation> {
        match self {
            Self::Passed(observation) => Some(observation),
            Self::Refused(_) => None,
        }
    }

    pub fn refusal(&self) -> Option<&RustSourceSyntaxError> {
        match self {
            Self::Passed(_) => None,
            Self::Refused(error) => Some(error),
        }
    }
}

pub fn evaluate_rust_source_syntax_frame(frame: &[u8]) -> RustSourceSyntaxOutcome {
    match evaluate(frame) {
        Ok(observation) => RustSourceSyntaxOutcome::Passed(observation),
        Err(error) => RustSourceSyntaxOutcome::Refused(error),
    }
}

pub(crate) fn trusted_rust_source_execution_observed(
    frame: Option<&[u8]>,
    exit_code: i32,
    stdout: &[u8],
    stderr_is_empty: bool,
) -> bool {
    let Some(frame) = frame else {
        return false;
    };
    let RustSourceSyntaxOutcome::Passed(observation) = evaluate_rust_source_syntax_frame(frame)
    else {
        return false;
    };
    let mut expected = rust_source_syntax_observation_json(&observation);
    expected.push(b'\n');
    exit_code == 0 && stdout == expected && stderr_is_empty
}

fn evaluate(frame: &[u8]) -> Result<RustSourceSyntaxObservation, RustSourceSyntaxError> {
    let sources = parse_frame(frame)?;
    let mut total_source_bytes = 0_u64;
    let mut bindings = Sha256::new();
    for source in &sources {
        let text = std::str::from_utf8(source.bytes)
            .map_err(|_| error(RustSourceSyntaxErrorKind::InvalidSourceUtf8))?;
        syn::parse_file(text).map_err(|_| error(RustSourceSyntaxErrorKind::InvalidRustSyntax))?;
        total_source_bytes = total_source_bytes
            .checked_add(source.bytes.len() as u64)
            .ok_or_else(|| error(RustSourceSyntaxErrorKind::IntegerOverflow))?;
        bindings.update((source.path.len() as u64).to_be_bytes());
        bindings.update(source.path.as_bytes());
        bindings.update(source.digest);
        bindings.update((source.bytes.len() as u64).to_be_bytes());
    }
    Ok(RustSourceSyntaxObservation {
        frame_sha256: digest(frame),
        source_bindings_sha256: format!("sha256:{:x}", bindings.finalize()),
        source_count: sources.len(),
        total_source_bytes,
    })
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod execution_observation_tests {
    use super::*;
    use crate::routine_work::{RustSourceFrameInput, encode_rust_source_syntax_frame};

    fn frame_and_stdout() -> (Vec<u8>, Vec<u8>) {
        let source = b"pub fn value() -> u8 { 1 }\n";
        let digest = format!("sha256:{:x}", Sha256::digest(source));
        let frame = encode_rust_source_syntax_frame(&[RustSourceFrameInput::new(
            "src/lib.rs",
            &digest,
            source.len() as u64,
            source,
        )])
        .unwrap();
        let RustSourceSyntaxOutcome::Passed(observation) =
            evaluate_rust_source_syntax_frame(&frame)
        else {
            panic!("canonical frame refused");
        };
        let mut stdout = rust_source_syntax_observation_json(&observation);
        stdout.push(b'\n');
        (frame, stdout)
    }

    #[test]
    fn only_exact_framed_exit_zero_observation_passes() {
        let (frame, stdout) = frame_and_stdout();
        assert!(trusted_rust_source_execution_observed(
            Some(&frame),
            0,
            &stdout,
            true
        ));
        assert!(!trusted_rust_source_execution_observed(
            None, 0, &stdout, true
        ));
        assert!(!trusted_rust_source_execution_observed(
            Some(&frame),
            0,
            b"{}\n",
            true
        ));
        assert!(!trusted_rust_source_execution_observed(
            Some(&frame),
            0,
            &stdout,
            false
        ));
        assert!(!trusted_rust_source_execution_observed(
            Some(&frame),
            7,
            &stdout,
            true
        ));
    }
}
