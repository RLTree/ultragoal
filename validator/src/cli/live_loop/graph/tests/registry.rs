use crate::cli::live_loop::surfaces;
use crate::scheduler::TaskClass;

#[test]
fn high_frequency_registry_covers_routine_hot_validation() {
    let ids = super::super::required_high_frequency_validation_ids();
    for required in [
        "line_caps_check",
        "namespace_check",
        "schema_validation",
        "package_inventory",
        "live_loop_measurement_rust_tests",
        "fmt_check",
        "build_check",
    ] {
        assert!(ids.contains(&required), "missing {required}: {ids:?}");
    }
}

#[test]
fn high_frequency_registry_classifies_authority_artifact_writers_as_serial() {
    for id in [
        "build_check",
        "live_loop_measurement_rust_tests",
        "line_caps_check",
        "namespace_check",
        "schema_validation",
        "package_inventory",
    ] {
        let surface = surfaces::surface_by_id(id).expect(id);
        assert_eq!(
            surface.execution_task_class,
            TaskClass::SharedAuthorityWriteSerial,
            "{id} writes authority artifacts and must not be executed by a parallel worker"
        );
        assert_ne!(surface.execution_serial_reason, "none", "{id}");
    }
}

#[test]
fn boundary_registry_keeps_broad_proof_surfaces_serial_but_not_hot_blocking() {
    for id in [
        "mandatory_law_validation",
        "source_obligations_check",
        "foundational_trace_check",
        "coverage_prove",
        "coverage_full_script",
        "coverage_fast_script",
        "source_audit",
        "red_fixture_report",
        "scripts_check",
        "touched_fixture_reports",
    ] {
        let surface = surfaces::surface_by_id(id).expect(id);
        assert_eq!(
            surface.execution_task_class,
            TaskClass::SharedAuthorityWriteSerial,
            "{id} writes authority artifacts and must not be executed by a parallel worker"
        );
        assert!(
            !surface.high_frequency,
            "{id} must not block dirty hot-loop validation"
        );
        assert_eq!(surface.hot_loop_policy, surfaces::BOUNDARY_PROOF_POLICY);
        assert_ne!(surface.execution_serial_reason, "none", "{id}");
    }
}

#[test]
fn read_only_high_frequency_registry_entries_remain_parallel() {
    let fmt = surfaces::surface_by_id("fmt_check").expect("fmt");
    assert_eq!(fmt.execution_task_class, TaskClass::PureReadParallel);
    assert_eq!(fmt.execution_serial_reason, "none");
}
