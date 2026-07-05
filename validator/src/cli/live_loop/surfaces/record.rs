use crate::scheduler::TaskClass;

pub(crate) const SAME_CANDIDATE: &str = "same_candidate_observed";
pub(crate) const ROUNDTRIP_REQUIRED: &str = "requires_command_telemetry_roundtrip";
const AUTHORITY_ARTIFACT_WRITE_REASON: &str =
    "writes canonical validation_artifacts or build artifacts requiring serial authority control";

#[derive(Clone, Copy)]
pub(crate) struct LoopValidationSurface {
    pub(crate) id: &'static str,
    pub(crate) surface: &'static str,
    pub(crate) command: &'static str,
    pub(crate) canonical_full_command: &'static str,
    pub(crate) narrow_rerun: &'static str,
    pub(crate) telemetry_reconciliation_state: &'static str,
    pub(crate) execution_task_class: TaskClass,
    pub(crate) execution_serial_reason: &'static str,
    pub(crate) high_frequency: bool,
}

const fn validation_surface_record(
    id: &'static str,
    surface: &'static str,
    command: &'static str,
    canonical_full_command: &'static str,
    narrow_rerun: &'static str,
    telemetry_reconciliation_state: &'static str,
    execution_task_class: TaskClass,
    execution_serial_reason: &'static str,
    high_frequency: bool,
) -> LoopValidationSurface {
    LoopValidationSurface {
        id,
        surface,
        command,
        canonical_full_command,
        narrow_rerun,
        telemetry_reconciliation_state,
        execution_task_class,
        execution_serial_reason,
        high_frequency,
    }
}

pub(crate) const fn context_read_surface(
    id: &'static str,
    surface: &'static str,
    command: &'static str,
    canonical_full_command: &'static str,
    narrow_rerun: &'static str,
    telemetry_reconciliation_state: &'static str,
) -> LoopValidationSurface {
    validation_surface_record(
        id,
        surface,
        command,
        canonical_full_command,
        narrow_rerun,
        telemetry_reconciliation_state,
        TaskClass::PureReadParallel,
        "none",
        false,
    )
}

pub(crate) const fn context_authority_artifact_surface(
    id: &'static str,
    surface: &'static str,
    command: &'static str,
    canonical_full_command: &'static str,
    narrow_rerun: &'static str,
    telemetry_reconciliation_state: &'static str,
) -> LoopValidationSurface {
    validation_surface_record(
        id,
        surface,
        command,
        canonical_full_command,
        narrow_rerun,
        telemetry_reconciliation_state,
        TaskClass::SharedAuthorityWriteSerial,
        AUTHORITY_ARTIFACT_WRITE_REASON,
        false,
    )
}

pub(crate) const fn hot_loop_read_surface(
    id: &'static str,
    surface: &'static str,
    command: &'static str,
    canonical_full_command: &'static str,
    narrow_rerun: &'static str,
) -> LoopValidationSurface {
    validation_surface_record(
        id,
        surface,
        command,
        canonical_full_command,
        narrow_rerun,
        ROUNDTRIP_REQUIRED,
        TaskClass::PureReadParallel,
        "none",
        true,
    )
}

pub(crate) const fn hot_loop_authority_artifact_surface(
    id: &'static str,
    surface: &'static str,
    command: &'static str,
    canonical_full_command: &'static str,
    narrow_rerun: &'static str,
) -> LoopValidationSurface {
    validation_surface_record(
        id,
        surface,
        command,
        canonical_full_command,
        narrow_rerun,
        ROUNDTRIP_REQUIRED,
        TaskClass::SharedAuthorityWriteSerial,
        AUTHORITY_ARTIFACT_WRITE_REASON,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_record_constructors_encode_execution_authority() {
        let context = context_read_surface(
            "candidate_delta",
            "candidate_delta",
            "git status",
            "git status",
            "git status",
            SAME_CANDIDATE,
        );
        assert_eq!(context.id, "candidate_delta");
        assert_eq!(context.execution_task_class, TaskClass::PureReadParallel);
        assert_eq!(context.execution_serial_reason, "none");
        assert!(!context.high_frequency);

        let context_writer = context_authority_artifact_surface(
            "package_digest",
            "package_boundary",
            "ultragoal package digest",
            "target/debug/ultragoal --root . package digest",
            "target/debug/ultragoal --root . package digest",
            SAME_CANDIDATE,
        );
        assert_eq!(
            context_writer.execution_task_class,
            TaskClass::SharedAuthorityWriteSerial
        );
        assert!(
            context_writer
                .execution_serial_reason
                .contains("canonical validation_artifacts")
        );
        assert!(!context_writer.high_frequency);

        let hot_loop_read = hot_loop_read_surface(
            "fmt_check",
            "rust_format",
            "cargo fmt --all --check",
            "cargo fmt --all --check",
            "cargo fmt --all --check",
        );
        assert_eq!(
            hot_loop_read.telemetry_reconciliation_state,
            ROUNDTRIP_REQUIRED
        );
        assert_eq!(
            hot_loop_read.execution_task_class,
            TaskClass::PureReadParallel
        );
        assert!(hot_loop_read.high_frequency);

        let hot_loop_writer = hot_loop_authority_artifact_surface(
            "source_audit",
            "source_audit",
            "ultragoal source audit",
            "target/debug/ultragoal --root . source audit",
            "target/debug/ultragoal --root . source audit",
        );
        assert_eq!(
            hot_loop_writer.execution_task_class,
            TaskClass::SharedAuthorityWriteSerial
        );
        assert!(hot_loop_writer.high_frequency);
    }
}
