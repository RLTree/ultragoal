//! Closed routine behaviors whose result is derived from trusted observations.

mod child_capability;
mod rust_source_frame;
mod rust_source_syntax;

pub(crate) use child_capability::{
    CHILD_MODE_ENV, CHILD_MODE_VALUE, LEGACY_BEHAVIOR_SELECTOR_ENV, LEGACY_CHILD_SELECTOR_ENV,
};
pub use rust_source_frame::{RustSourceFrameInput, encode_rust_source_syntax_frame};
pub(crate) use rust_source_syntax::trusted_rust_source_execution_observed;
pub use rust_source_syntax::{
    RustSourceSyntaxError, RustSourceSyntaxErrorKind, RustSourceSyntaxObservation,
    RustSourceSyntaxOutcome, evaluate_rust_source_syntax_frame,
    rust_source_syntax_observation_json,
};
