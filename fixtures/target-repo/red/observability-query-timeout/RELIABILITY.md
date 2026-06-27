# RELIABILITY

Reliability means a run, workflow, service, or agent task can be stopped,
inspected, resumed, retried, rejected, or closed without losing truth.

## Retry, Timeout, And Idempotency

- Retry only errors documented as retryable by the calling layer.
- Do not retry parser failures, authorization failures, hash mismatches, or
  policy denials as if they were transient.
- Use bounded attempts, backoff, and jitter for external or model/tool calls.
- Side effects must be idempotent or guarded by durable state so a resumed run
  cannot double-apply a decision.
- Cancellation and cleanup must preserve a receipt, ledger row, or blocker.

## Fanout And Concurrency

- Bounded fanout is the default. Document caps for agents, workers, queues,
  ports, caches, and state roots.
- Parallel work must use isolated workspaces and isolated mutable state.
- Closeout must preserve evidence before teardown.

## Feedback Latency

Name the fast gate, focused changed-slice gate, full package gate, and any
latency target. Measure before changing caches, parallelism, cold-start
behavior, or gate scope. Do not lower correctness, security, or coverage to win
timing.

## Recovery

Name state roots, append-only ledgers, mutable summaries, backup paths, and
safe cleanup commands. Tests must isolate real state roots.
