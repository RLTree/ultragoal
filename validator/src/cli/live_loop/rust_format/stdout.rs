pub(super) struct FormatResult {
    pub(super) status: &'static str,
    pub(super) exit_code: i32,
    mode: &'static str,
    work_unit_count: usize,
    duration_ms: u64,
    validation_summary: String,
    failure_class: &'static str,
    where_failed: &'static str,
    why_failed: String,
    next_repair: &'static str,
    stdout_digest: String,
    stderr_digest: String,
}

impl FormatResult {
    pub(super) fn pass(
        mode: &'static str,
        work_unit_count: usize,
        duration_ms: u64,
        validation_summary: &'static str,
    ) -> Self {
        Self {
            status: "pass",
            exit_code: 0,
            mode,
            work_unit_count,
            duration_ms,
            validation_summary: validation_summary.to_string(),
            failure_class: "none",
            where_failed: "none",
            why_failed: "none".to_string(),
            next_repair: "none",
            stdout_digest: crate::digest::bytes(&[]),
            stderr_digest: crate::digest::bytes(&[]),
        }
    }

    pub(super) fn fail(
        mode: &'static str,
        work_unit_count: usize,
        duration_ms: u64,
        exit_code: i32,
        failure_class: &'static str,
        where_failed: &'static str,
        next_repair: &'static str,
        stdout: &[u8],
        stderr: &[u8],
    ) -> Self {
        Self {
            status: "fail",
            exit_code,
            mode,
            work_unit_count,
            duration_ms,
            validation_summary: "routine Rust formatting command failed".to_string(),
            failure_class,
            where_failed,
            why_failed: format!("routine Rust formatting command exited {exit_code}"),
            next_repair,
            stdout_digest: crate::digest::bytes(stdout),
            stderr_digest: crate::digest::bytes(stderr),
        }
    }

    pub(super) fn blocked(
        mode: &'static str,
        duration_ms: u64,
        failure_class: &'static str,
        where_failed: &'static str,
        why_failed: String,
        next_repair: &'static str,
    ) -> Self {
        Self {
            status: "blocked",
            exit_code: 1,
            mode,
            work_unit_count: 0,
            duration_ms,
            validation_summary: "routine Rust formatting changed-input discovery was unavailable"
                .to_string(),
            failure_class,
            where_failed,
            why_failed,
            next_repair,
            stdout_digest: crate::digest::bytes(&[]),
            stderr_digest: crate::digest::bytes(&[]),
        }
    }

    pub(super) fn stdout_line(&self) -> String {
        format!(
            "ultragoal-loop-format-check {} mode={} rustfmt_edition={} changed_rust_file_count={} work_unit_count={} duration_ms={} validation_summary='{}' failure_class={} where_failed={} why_failed='{}' next_repair='{}' stdout_digest={} stderr_digest={} claim_ceiling='source-local routine repair only; strict formatting proof remains cargo fmt --all --check'",
            self.status,
            self.mode,
            super::RUSTFMT_EDITION,
            self.work_unit_count,
            self.work_unit_count,
            self.duration_ms,
            self.validation_summary,
            self.failure_class,
            self.where_failed,
            self.why_failed,
            self.next_repair,
            self.stdout_digest,
            self.stderr_digest
        )
    }
}
