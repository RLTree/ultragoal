use super::record::MeasurementExecutionAuthority;
use crate::cli::live_loop::surfaces::surface_by_id;

#[test]
fn measurement_authority_is_derived_from_surface_and_batch_position() {
    let fmt = surface_by_id("fmt_check").expect("fmt surface");
    let build = surface_by_id("build_check").expect("build surface");

    let fmt_authority = MeasurementExecutionAuthority::from_measurement_batch(fmt, 3, 1);
    let build_authority = MeasurementExecutionAuthority::from_measurement_batch(build, 3, 0);

    assert_eq!(fmt_authority.execution_task_class, "pure_read_parallel");
    assert_eq!(fmt_authority.execution_serial_reason, "none");
    assert_eq!(fmt_authority.task_count, 3);
    assert_eq!(fmt_authority.queue_depth, 2);
    assert_eq!(
        build_authority.execution_task_class,
        "shared_authority_write_serial"
    );
    let reason = build_authority.execution_serial_reason;
    assert!(reason.starts_with("writes"));
    assert_eq!(build_authority.worker_count, 1);
}
