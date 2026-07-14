//! Closed routine behaviors whose result is derived from trusted observations.

mod child_capability;
mod rust_source_frame;
mod rust_source_syntax;

pub(crate) use child_capability::{
    CHILD_CAPABILITY_ENV, CHILD_CAPABILITY_FD, MAX_CHILD_CAPABILITY_BYTES, RoutineChildCapability,
};
pub use rust_source_frame::{RustSourceFrameInput, encode_rust_source_syntax_frame};
pub(crate) use rust_source_syntax::trusted_rust_source_execution_observed;
pub use rust_source_syntax::{
    RustSourceSyntaxError, RustSourceSyntaxErrorKind, RustSourceSyntaxObservation,
    RustSourceSyntaxOutcome, evaluate_rust_source_syntax_frame,
    rust_source_syntax_observation_json,
};
