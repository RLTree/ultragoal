use serde_json::Value;
use std::collections::BTreeSet;

pub(super) struct RequiredInput {
    pub(super) source_id: &'static str,
    pub(super) url: &'static str,
}

pub(super) const REQUIRED: &[RequiredInput] = &[
    RequiredInput {
        source_id: "openai-harness-engineering",
        url: "https://openai.com/index/harness-engineering/",
    },
    RequiredInput {
        source_id: "openai-codex-iterative-repair-loops",
        url: "https://developers.openai.com/cookbook/examples/codex/build_iterative_repair_loops_with_codex",
    },
    RequiredInput {
        source_id: "openai-agents-observability",
        url: "https://developers.openai.com/api/docs/guides/agents/integrations-observability",
    },
    RequiredInput {
        source_id: "openai-agents-sdk-tracing",
        url: "https://openai.github.io/openai-agents-python/tracing/",
    },
    RequiredInput {
        source_id: "google-sre-monitoring-workbook",
        url: "https://sre.google/workbook/monitoring/",
    },
    RequiredInput {
        source_id: "google-sre-four-golden-signals",
        url: "https://sre.google/sre-book/monitoring-distributed-systems/",
    },
    RequiredInput {
        source_id: "charity-majors-structured-events",
        url: "https://charity.wtf/2022/08/15/live-your-best-life-with-structured-events/",
    },
    RequiredInput {
        source_id: "honeycomb-high-cardinality",
        url: "https://docs.honeycomb.io/get-started/observability/concepts/high-cardinality/",
    },
    RequiredInput {
        source_id: "opentelemetry-semantic-conventions",
        url: "https://opentelemetry.io/docs/concepts/semantic-conventions/",
    },
    RequiredInput {
        source_id: "openai-agent-improvement-loop",
        url: "https://developers.openai.com/cookbook/examples/agents_sdk/agent_improvement_loop",
    },
    RequiredInput {
        source_id: "openai-self-improving-tax-agent",
        url: "https://openai.com/index/building-self-improving-tax-agents-with-codex/",
    },
    RequiredInput {
        source_id: "attached-rust-devx-guide",
        url: "codex-attachment://rust-devx-guide",
    },
    RequiredInput {
        source_id: "attached-typescript-frontend-guide",
        url: "codex-attachment://typescript-frontend-guide",
    },
    RequiredInput {
        source_id: "agentic-gold-standard-stack-synthesis-2026-07-01",
        url: "codex-attachment://agentic-gold-standard-stack-synthesis-2026-07-01",
    },
    RequiredInput {
        source_id: "gold-standard-stack-developer-experience-governance-2026-07-01",
        url: "codex-attachment://gold-standard-stack-developer-experience-governance-2026-07-01",
    },
];

pub(super) fn check(value: &Value, out: &mut Vec<String>) {
    let Some(rows) = value
        .pointer("/operating_loop/research_inputs")
        .and_then(Value::as_array)
    else {
        out.push("observability_operating_research_inputs_missing".to_string());
        return;
    };
    let mut seen = BTreeSet::new();
    for row in rows {
        let source_id = text(row, "source_id");
        if source_id.is_empty() {
            out.push("observability_operating_research_input_source_id_missing".to_string());
            continue;
        }
        if !seen.insert(source_id.to_string()) {
            out.push(format!(
                "observability_operating_research_input_duplicate:{source_id}"
            ));
        }
        let url = text(row, "url");
        match required(source_id) {
            Some(expected) if url != expected.url => out.push(format!(
                "observability_operating_research_input_url_mismatch:{source_id}"
            )),
            Some(_) => {}
            None => out.push(format!(
                "observability_operating_research_input_unknown:{source_id}"
            )),
        }
        if text(row, "repo_rule").is_empty() {
            out.push(format!(
                "observability_operating_research_input_rule_missing:{source_id}"
            ));
        }
    }
    for required in REQUIRED {
        if !seen.contains(required.source_id) {
            out.push(format!(
                "observability_operating_research_input_missing:{}",
                required.source_id
            ));
        }
    }
}

fn required(source_id: &str) -> Option<&'static RequiredInput> {
    REQUIRED.iter().find(|row| row.source_id == source_id)
}

fn text<'a>(row: &'a Value, key: &str) -> &'a str {
    row.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
}

#[cfg(test)]
pub(super) fn fixture_inputs() -> Value {
    serde_json::json!(
        REQUIRED
            .iter()
            .map(|row| {
                serde_json::json!({
                    "source_id": row.source_id,
                    "source": row.source_id,
                    "url": row.url,
                    "repo_rule": "test fixture research rule"
                })
            })
            .collect::<Vec<_>>()
    )
}
