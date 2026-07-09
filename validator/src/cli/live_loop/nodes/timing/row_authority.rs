use super::record_fields;
use serde_json::Value;

pub(crate) const SOURCE_LOCAL_CLAIM_CEILING: &str = "source-local loop timing only; readiness release completion final-packet and update_goal remain blocked";
const SOURCE_LOCAL_CLAIM_IMPACT: &str =
    "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal";

pub(crate) struct VerifiedLocalDigests {
    pub(crate) result_digest: String,
    pub(crate) output_digest: String,
    pub(crate) verified_local_result_digest: String,
    pub(crate) verified_local_output_digest: String,
}

pub(crate) fn verified_local_digests(row: &Value) -> Option<VerifiedLocalDigests> {
    let stdout_digest =
        record_fields::valid_digest(record_fields::text(row, "verified_local_stdout_digest")?)?;
    let stderr_digest =
        record_fields::valid_digest(record_fields::text(row, "verified_local_stderr_digest")?)?;
    let output_digest = record_fields::valid_digest(record_fields::text(row, "output_digest")?)?;
    let verified_local_output_digest =
        record_fields::valid_digest(record_fields::text(row, "verified_local_output_digest")?)?;
    let result_digest = record_fields::valid_digest(record_fields::text(row, "result_digest")?)?;
    let verified_local_result_digest =
        record_fields::valid_digest(record_fields::text(row, "verified_local_result_digest")?)?;
    let expected_output_digest = expected_output_digest(stdout_digest, stderr_digest);
    let expected_result_digest = expected_result_digest(row, &expected_output_digest)?;
    (output_digest == expected_output_digest
        && verified_local_output_digest == expected_output_digest
        && result_digest == expected_result_digest
        && verified_local_result_digest == expected_result_digest)
        .then(|| VerifiedLocalDigests {
            result_digest: result_digest.to_string(),
            output_digest: output_digest.to_string(),
            verified_local_result_digest: verified_local_result_digest.to_string(),
            verified_local_output_digest: verified_local_output_digest.to_string(),
        })
}

pub(crate) fn rust_test_count_is_claim_safe(row: &Value, node_id: &str) -> bool {
    if node_id != "live_loop_measurement_rust_tests" {
        return true;
    }
    row.get("verified_local_executed_test_count")
        .and_then(Value::as_u64)
        .is_some_and(|count| count > 0)
}

pub(crate) fn claim_ceiling_is_source_local(row: &Value) -> bool {
    record_fields::text(row, "claim_ceiling") == Some(SOURCE_LOCAL_CLAIM_CEILING)
        && record_fields::text(row, "claim_impact") == Some(SOURCE_LOCAL_CLAIM_IMPACT)
}

fn expected_output_digest(stdout_digest: &str, stderr_digest: &str) -> String {
    crate::digest::bytes(format!("stdout={stdout_digest};stderr={stderr_digest}").as_bytes())
}

fn expected_result_digest(row: &Value, output_digest: &str) -> Option<String> {
    let exit_code = record_fields::i32_field(row, "exit_status")?;
    let launch_error = row.get("verified_local_launch_error")?.as_bool()?;
    Some(crate::digest::bytes(
        format!("exit={exit_code};launch={launch_error};output={output_digest}").as_bytes(),
    ))
}
