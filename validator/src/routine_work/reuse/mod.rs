mod artifact;
mod decision;
mod expectation;
mod model;
mod receipt;
mod witness;

pub use artifact::{capture_executed_result, observe_result_artifact};
pub use decision::assess_reuse;
pub use model::{DependencyResult, ReceiptState, ReuseExpectation, ReuseMiss, RunOutcome};
pub use receipt::ReuseReceipt;
pub use witness::{CapturedExecution, ExecutedWork, ObservedResult, ReuseDecision, VerifiedReuse};
