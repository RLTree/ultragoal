mod artifact;
mod decision;
mod expectation;
mod receipt;
mod reuse_record;
mod witness;

pub use artifact::{capture_executed_result, observe_result_artifact};
pub use decision::assess_reuse;
pub use receipt::ReuseReceipt;
pub use reuse_record::{DependencyResult, ReceiptState, ReuseExpectation, ReuseMiss, RunOutcome};
pub use witness::{CapturedExecution, ExecutedWork, ObservedResult, ReuseDecision, VerifiedReuse};
