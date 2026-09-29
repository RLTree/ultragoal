# Agent Observability Stack

The harness-engineering article treats observability as part of the agent work
surface: each worktree can run an isolated app instance, emit logs, metrics, and
traces, and let Codex query those signals while it works. This package adapts
that idea into a portable repo contract.

## Portable Baseline

When the user requests observability setup, the goal contract requires it, or a runtime/performance/UI claim cites observability, the target repo should expose this agent-useful surface:

```text
docs/observability.md
scripts/observe                         # query events/context for agents
validation_artifacts/observability/
validation_artifacts/observability/events.jsonl
validation_artifacts/observability/agent-context.md
```

The baseline is intentionally small. It does not require Grafana, Loki,
Prometheus, Tempo, OpenTelemetry, Docker, or cloud credentials. If those already
exist, the repo should expose them through `scripts/observe` rather than
duplicating their storage. The successor routine check validates this query surface
statically by default so a marker plus event file cannot masquerade as an
agent-usable observability stack without executing target-owned code.

## Required Events

`events.jsonl` is append-only. Each row should include the repo, commit,
worktree, actor, command or journey id, event kind, timestamp, status, and any
artifact digest. It is a context ledger for agents, not a product analytics
stream.

Recommended event kinds:

- `run_started`
- `run_completed`
- `run_failed`
- `port_allocated`
- `journey_started`
- `journey_completed`
- `metric_observed`
- `log_query_observed`
- `trace_observed`
- `blocker_recorded`

## Gate Marker

A target repo gate that claims harness observability must emit:

```text
harness-check:observability pass
```

The marker is accepted only with the baseline files or a documented stronger
adapter exposed through `scripts/observe`. The successor routine check treats missing
observability as a failure only when observability is explicitly required by
flag, goal contract, or claim dependency; otherwise it records that
observability was not requested.

## Claim Ceiling

Observability can support runtime, UI, performance, and reliability claims only
when the cited receipt contains non-zero byte-checked artifact digests and a
static/read-only `scripts/observe` surface exists. Running target-owned
observability adapters is a later trusted-execution phase, not part of the
default package/static/fixture proof ceiling. Missing credentials, disabled
local runtime, or unavailable backends become blockers.
