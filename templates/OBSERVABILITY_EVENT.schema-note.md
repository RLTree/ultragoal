# Observability Event Ledger Shape

Use append-only JSONL at `validation_artifacts/observability/events.jsonl`.
Each row should include at least:

```json
{
  "schema": "harness-ultragoal.observability-event.v1",
  "repo": "/absolute/repo/path",
  "commit": "abcdef0",
  "worktree": "/absolute/worktree/path",
  "actor_id": "agent-or-user",
  "event_kind": "run_completed",
  "target_id": "check-or-journey-id",
  "status": "pass",
  "observed_at": "2026-06-16T00:00:00Z",
  "artifact": {
    "path": "validation_artifacts/observability/check.json",
    "digest": "sha256:..."
  }
}
```

Keep credentials and raw secrets out of the ledger.
