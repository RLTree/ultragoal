extern crate routine_surface;

use routine_surface::routine_work::{
    CapturedExecution, DependencyResult, ExecutedWork, ObservedResult, ReportStatus, RoutineReport,
    VerifiedReuse,
};

fn reconstruct(
    executed: ExecutedWork,
    capture_run_sha256: String,
    observed: ObservedResult,
    reused: VerifiedReuse,
    dependency: DependencyResult,
    result_sha256: String,
    captured: CapturedExecution,
    report: RoutineReport,
) {
    let _ = ExecutedWork {
        capture_run_sha256,
        ..executed
    };
    let ObservedResult { facts: _ } = observed;
    let VerifiedReuse { facts: _ } = reused;
    let _ = DependencyResult {
        result_sha256,
        ..dependency
    };
    let CapturedExecution {
        work: _,
        receipt_json: _,
    } = captured;
    let _ = RoutineReport {
        status: ReportStatus::CompleteExecution,
        ..report
    };
}

fn main() {
    let _ = reconstruct;
}
