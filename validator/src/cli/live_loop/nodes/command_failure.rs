use serde_json::{Value, json};

const MAX_FIELD_BYTES: usize = 1600;

#[derive(Clone, Debug, Default)]
pub(crate) struct CommandFailureSummary {
    pub(crate) failed_law: Option<String>,
    pub(crate) failed_check: Option<String>,
    pub(crate) why_failed: Option<String>,
    pub(crate) where_failed: Option<String>,
    pub(crate) claim_impact: Option<String>,
    pub(crate) next_repair: Option<String>,
    pub(crate) receipt: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) correlation_id: Option<String>,
    pub(crate) query_logs: Option<String>,
    pub(crate) query_metrics: Option<String>,
    pub(crate) query_traces: Option<String>,
}

impl CommandFailureSummary {
    pub(crate) fn from_stdout(stdout: &[u8]) -> Self {
        let text = String::from_utf8_lossy(stdout);
        let failure_text = failure_detail_line(&text);
        Self {
            failed_law: between(failure_text, "failed_law=", " failed_check="),
            failed_check: between(failure_text, "failed_check=", " why="),
            why_failed: between(failure_text, "why=", " where="),
            where_failed: between(failure_text, "where=", " claim_impact="),
            claim_impact: between(failure_text, "claim_impact=", " next_repair="),
            next_repair: between(failure_text, "next_repair=", " receipt="),
            receipt: between(failure_text, "receipt=", " run_id="),
            run_id: token_after(failure_text, "run_id="),
            correlation_id: token_after(failure_text, "correlation_id="),
            query_logs: between(failure_text, "query_logs='", "' query_metrics="),
            query_metrics: between(failure_text, "query_metrics='", "' query_traces="),
            query_traces: between(failure_text, "query_traces='", "'"),
        }
    }

    pub(crate) fn from_value(value: Option<&Value>) -> Self {
        let Some(value) = value else {
            return Self::default();
        };
        Self {
            failed_law: text(value, "failed_law"),
            failed_check: text(value, "failed_check"),
            why_failed: text(value, "why_failed"),
            where_failed: text(value, "where_failed"),
            claim_impact: text(value, "claim_impact"),
            next_repair: text(value, "next_repair"),
            receipt: text(value, "receipt"),
            run_id: text(value, "run_id"),
            correlation_id: text(value, "correlation_id"),
            query_logs: text(value, "query_logs"),
            query_metrics: text(value, "query_metrics"),
            query_traces: text(value, "query_traces"),
        }
    }

    pub(crate) fn to_value(&self) -> Value {
        json!({
            "failed_law": self.failed_law,
            "failed_check": self.failed_check,
            "why_failed": self.why_failed,
            "where_failed": self.where_failed,
            "claim_impact": self.claim_impact,
            "next_repair": self.next_repair,
            "receipt": self.receipt,
            "run_id": self.run_id,
            "correlation_id": self.correlation_id,
            "query_logs": self.query_logs,
            "query_metrics": self.query_metrics,
            "query_traces": self.query_traces
        })
    }
}

fn failure_detail_line(text: &str) -> &str {
    text.lines()
        .find(|line| line.contains("failed_law="))
        .unwrap_or(text)
}

fn between(text: &str, start: &str, end: &str) -> Option<String> {
    let start_index = text.find(start)? + start.len();
    let rest = &text[start_index..];
    let end_index = rest.find(end).unwrap_or(rest.len());
    bounded(&rest[..end_index])
}

fn token_after(text: &str, start: &str) -> Option<String> {
    let start_index = text.find(start)? + start.len();
    let rest = &text[start_index..];
    let token = rest.split_whitespace().next()?;
    bounded(token)
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).and_then(bounded)
}

fn bounded(raw: &str) -> Option<String> {
    let cleaned = raw.replace(['\r', '\n'], " ");
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut out = String::new();
    for ch in trimmed.chars() {
        if out.len() + ch.len_utf8() > MAX_FIELD_BYTES {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::CommandFailureSummary;
    use serde_json::json;

    #[test]
    fn command_failure_summary_extracts_agent_actionable_stdout_fields() {
        let stdout = b"ultragoal-mandatory-law-validation fail candidate=sha256:current \
failed_law=full-local-observability-stack-integration-non-opaque-failure \
failed_check=mandatory-law-validation-observability-binding \
why=mandatory law validation failed: stale receipt where=mandatory-law.validation \
claim_impact=blocks_readiness next_repair=repair stale receipt, then rerun mandatory-law validation \
receipt=validation_artifacts/observability/mandatory-law-validation.json run_id=run-123 \
correlation_id=corr-456 query_logs='ultragoal observe logs query --run-id run-123' \
query_metrics='ultragoal observe metrics query --query \"bounded\" --limit 100' \
query_traces='ultragoal observe traces query --run-id run-123'";
        let summary = CommandFailureSummary::from_stdout(stdout);
        assert_eq!(
            summary.failed_law.as_deref(),
            Some("full-local-observability-stack-integration-non-opaque-failure")
        );
        assert_eq!(
            summary.where_failed.as_deref(),
            Some("mandatory-law.validation")
        );
        assert!(
            summary
                .why_failed
                .as_deref()
                .expect("why")
                .contains("stale receipt")
        );
        assert!(
            summary
                .query_metrics
                .as_deref()
                .expect("metrics")
                .contains("--limit 100")
        );
        assert!(summary.why_failed.is_some());
        assert!(summary.next_repair.is_some());
    }

    #[test]
    fn command_failure_summary_uses_failure_detail_line_for_repair_ids() {
        let stdout = b"ultragoal-mandatory-law-validation fail operation=mandatory-law.validation candidate=sha256:current receipt=validation_artifacts/observability/mandatory-law-validation.json run_id=run-summary correlation_id=corr-summary claim_impact=mandatory_law_validation_failed_blocks_readiness_release_completion_update_goal supported_claims=none unsupported_claims=completion,readiness\n\
failed_law=full-local-observability-stack-integration-non-opaque-failure failed_check=mandatory-law-validation-observability-binding why=mandatory law validation failed: stale evidence where=mandatory-law.validation claim_impact=mandatory_law_validation_failed_blocks_readiness_release_completion_update_goal next_repair=query this run through observe logs/metrics/traces, repair stale evidence, then rerun mandatory-law validation receipt=validation_artifacts/observability/mandatory-law-validation.json run_id=run-detail correlation_id=corr-detail query_logs='ultragoal observe logs query --run-id run-detail --limit 100' query_metrics='ultragoal observe metrics query --query bounded --limit 100' query_traces='ultragoal observe traces query --run-id run-detail --limit 100'";
        let summary = CommandFailureSummary::from_stdout(stdout);
        assert_eq!(summary.run_id.as_deref(), Some("run-detail"));
        assert_eq!(summary.correlation_id.as_deref(), Some("corr-detail"));
        assert_eq!(
            summary.claim_impact.as_deref(),
            Some("mandatory_law_validation_failed_blocks_readiness_release_completion_update_goal")
        );
        assert!(
            summary
                .query_logs
                .as_deref()
                .expect("query logs")
                .contains("run-detail")
        );
        assert!(
            !summary
                .correlation_id
                .as_deref()
                .expect("correlation")
                .contains("claim_impact"),
            "{summary:?}"
        );
    }

    #[test]
    fn command_failure_summary_reads_bounded_receipt_fields() {
        let long_next_repair = "repair-source ".repeat(200);
        let value = json!({
            "failed_law": "full-local-observability-stack-integration-non-opaque-failure",
            "failed_check": "mandatory-law-validation-observability-binding",
            "why_failed": "mandatory law validation failed\nbecause receipt is stale",
            "where_failed": "mandatory-law.validation",
            "claim_impact": "blocks_readiness",
            "next_repair": long_next_repair,
            "receipt": "validation_artifacts/observability/mandatory-law-validation.json",
            "run_id": "run-123",
            "correlation_id": "corr-456",
            "query_logs": "ultragoal observe logs query --run-id run-123",
            "query_metrics": "ultragoal observe metrics query --query bounded",
            "query_traces": "ultragoal observe traces query --run-id run-123"
        });
        let summary = CommandFailureSummary::from_value(Some(&value));
        assert_eq!(
            summary.failed_check.as_deref(),
            Some("mandatory-law-validation-observability-binding")
        );
        assert_eq!(
            summary.why_failed.as_deref(),
            Some("mandatory law validation failed because receipt is stale")
        );
        assert!(
            summary
                .next_repair
                .as_deref()
                .expect("repair")
                .ends_with("...")
        );
        assert_eq!(summary.run_id.as_deref(), Some("run-123"));
        assert_eq!(summary.correlation_id.as_deref(), Some("corr-456"));
        assert_eq!(
            summary.query_traces.as_deref(),
            Some("ultragoal observe traces query --run-id run-123")
        );
    }

    #[test]
    fn command_failure_summary_ignores_absent_and_empty_receipt_fields() {
        assert!(CommandFailureSummary::from_value(None).why_failed.is_none());
        let value = json!({
            "why_failed": "   ",
            "where_failed": "\n",
            "run_id": 17
        });
        let summary = CommandFailureSummary::from_value(Some(&value));
        assert!(summary.why_failed.is_none());
        assert!(summary.where_failed.is_none());
        assert!(summary.run_id.is_none());
    }
}
