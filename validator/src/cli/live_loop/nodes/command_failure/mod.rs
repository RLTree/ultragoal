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
            receipt: text(value, "receipt").or_else(|| text(value, "receipt_path")),
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

    pub(crate) fn has_details(&self) -> bool {
        self.failed_law.is_some()
            || self.failed_check.is_some()
            || self.why_failed.is_some()
            || self.where_failed.is_some()
            || self.claim_impact.is_some()
            || self.next_repair.is_some()
            || self.receipt.is_some()
            || self.run_id.is_some()
            || self.correlation_id.is_some()
            || self.query_logs.is_some()
            || self.query_metrics.is_some()
            || self.query_traces.is_some()
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
#[path = "tests.rs"]
mod tests;
