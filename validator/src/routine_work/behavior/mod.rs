//! Closed routine behaviors whose result is derived from trusted observations.

mod rust_source_frame;
mod rust_source_syntax;

pub use rust_source_frame::{RustSourceFrameInput, encode_rust_source_syntax_frame};
pub use rust_source_syntax::{
    RustSourceSyntaxError, RustSourceSyntaxErrorKind, RustSourceSyntaxObservation,
    RustSourceSyntaxOutcome, evaluate_rust_source_syntax_frame,
    rust_source_syntax_observation_json,
};
