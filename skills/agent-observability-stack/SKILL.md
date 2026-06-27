# Agent Observability Stack

Use this separate setup skill when a user, goal contract, or claim surface asks to install agent-facing observability for a repo, lane, or goal run needing runtime context beyond
static files: service startup, logs, traces, metrics, CLI timing, browser
journeys, workflow runs, or repeated agent handoffs.

## Contract

Observability is an agent-facing proof surface. It exists to let agents ask:
what ran, what failed, how long did it take, which user journey or workflow was
observed, and which artifact proves it? It must not become a dashboard project
unless the target product already needs one.

## Required Shape

When explicitly invoked for a target repo, install the smallest useful stack that provides:

1. Run ledger: append-only JSONL events for starts, exits, failures, retries,
   ports, pids, worktree, actor, command, commit, and artifact digests.
2. Log access: either queryable local log files or an explicit adapter to the
   repo's existing logging backend. The query command must be documented.
3. Metrics access: at minimum startup time, command duration, error count, and
   critical journey duration. If Prometheus or equivalent exists, document the
   PromQL entrypoint instead of inventing another metric format.
4. Trace access: for services or UIs with multi-step journeys, provide either
   local span receipts or a TraceQL-compatible backend pointer. A single CLI
   command does not need distributed tracing.
5. Agent context summary: a generated short report that names current runtime
   surfaces, failing spans/logs, latest proof artifacts, and known blind spots.

## Fresh Repo Setup

Add these files unless a stronger repo-native equivalent already exists:

- `docs/observability.md`
- `validation_artifacts/observability/events.jsonl`
- `validation_artifacts/observability/agent-context.md`
- `scripts/observe` or an existing gate subcommand documented in
  `docs/observability.md`

`scripts/check` must verify the observability files exist and must emit the
marker `harness-check:observability pass` after the check succeeds.
The default validator treats `scripts/observe` as an untrusted target-owned
surface and validates it statically; do not require agents to execute it during
package/static/fixture proof.

## Retrofit Setup

Do not replace existing production observability. Instead:

1. Inventory current logs, metrics, traces, dashboards, and local dev commands.
2. Add a thin agent adapter that names how to query them.
3. Record missing surfaces as explicit blockers, not fake receipts.
4. Keep credentials out of receipts; store only paths, commands, redacted query
   text, timestamps, and digests.

## Proof Requirements

A runtime or UI claim can cite observability only when the receipt includes:

- target repo and commit;
- worktree or run id;
- command or journey id;
- log/metric/trace query or local file path;
- non-zero digest for the returned artifact;
- claim-specific threshold or expected behavior;
- exact blocker when the backend is unavailable.

## Anti-Laziness Rules

- A screenshot without logs or runtime receipts is UI evidence only, not service
  observability.
- A log file path without a digest is not proof.
- A metric name without a query result is not proof.
- A trace/span claim without a span id, local receipt, or blocker is not proof.
- If the app cannot run locally, record the blocked command and do not claim
  runtime observability.
