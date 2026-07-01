use serde_json::json;

pub(super) fn inputs() -> serde_json::Value {
    json!([
        row(
            "openai-harness-engineering",
            "https://openai.com/index/harness-engineering/"
        ),
        row(
            "openai-codex-iterative-repair-loops",
            "https://developers.openai.com/cookbook/examples/codex/build_iterative_repair_loops_with_codex"
        ),
        row(
            "openai-agents-observability",
            "https://developers.openai.com/api/docs/guides/agents/integrations-observability"
        ),
        row(
            "openai-agents-sdk-tracing",
            "https://openai.github.io/openai-agents-python/tracing/"
        ),
        row(
            "google-sre-monitoring-workbook",
            "https://sre.google/workbook/monitoring/"
        ),
        row(
            "google-sre-four-golden-signals",
            "https://sre.google/sre-book/monitoring-distributed-systems/"
        ),
        row(
            "charity-majors-structured-events",
            "https://charity.wtf/2022/08/15/live-your-best-life-with-structured-events/"
        ),
        row(
            "honeycomb-high-cardinality",
            "https://docs.honeycomb.io/get-started/observability/concepts/high-cardinality/"
        ),
        row(
            "opentelemetry-semantic-conventions",
            "https://opentelemetry.io/docs/concepts/semantic-conventions/"
        ),
        row(
            "openai-agent-improvement-loop",
            "https://developers.openai.com/cookbook/examples/agents_sdk/agent_improvement_loop"
        ),
        row(
            "openai-self-improving-tax-agent",
            "https://openai.com/index/building-self-improving-tax-agents-with-codex/"
        ),
        row(
            "attached-rust-devx-guide",
            "codex-attachment://rust-devx-guide"
        ),
        row(
            "attached-typescript-frontend-guide",
            "codex-attachment://typescript-frontend-guide"
        )
    ])
}

fn row(source_id: &str, url: &str) -> serde_json::Value {
    json!({
        "source_id": source_id,
        "source": source_id,
        "url": url,
        "repo_rule": "test fixture research rule"
    })
}
