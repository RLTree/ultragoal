---
name: goal-run
description: "Orchestrate durable, dependency-closed Harness Ultragoal work under one root. Use for multi-scope goals, adaptive dependency graphs, disjoint worker leases, checkpoints, independent reviews, recovery after interruption, authority stops, or honest ceiling reporting."
---

# Goal Run

Coordinate one durable goal through root-owned planning, disjoint work packages,
independent review, immediate reconciliation, and recoverable checkpoints. Use
host goal state only when the host exposes it; never simulate it or merge it
with product state.

## Inputs

Require the exact objective and acceptance boundary, exposed host goal metadata,
current candidate identity, dependency graph, findings, worktrees, active
leases, available capabilities, authority owners, destructive decisions, and
proof boundaries. Record model, mode, reasoning, and runtime configuration only
when the runtime exposes them directly.

## Recompute before planning or recovery

Use read-only current surfaces:

```text
ultragoal --json inspect context
ultragoal --json inspect inventory
ultragoal --json inspect capabilities
ultragoal --json inspect findings
ultragoal --json inspect claims
ultragoal --json next
```

Recompute repository root, branch, HEAD, status, worktrees, candidate identity,
tooling, permissions, public command inventory, component inventory, active
leases, worker results, and reviews. A stale context, missing required state
surface, overlapping lease, or ambiguous owner blocks the dependent work
package; it does not authorize a historical registry or prompt-inferred state.

## Orchestrate

1. Build a dependency-closed graph and keep independent ready work active.
2. Retain shared manifests, dependencies, public commands, generated authority,
   migration, integration, claim decisions, release, and final reporting at the
   root.
3. Give each implementation worker one run-scoped lease disjoint in paths,
   semantic symbols, generated outputs, fixtures, scratch roots, and effects.
4. Require `WorkerResult-v1`. Workers may request root changes but never apply
   them or claim acceptance.
5. Freeze each candidate, assign a reviewer who did not implement it, reproduce
   evidence, and attack scope escape, stale identity, false passes, security,
   and recovery.
6. Integrate accepted work immediately; issue a corrective lease for material
   rework. Recompute every affected digest and claim ceiling.
7. Checkpoint the candidate, accepted sets, preserved state, open findings,
   blockers, and one exact next action after every material change or recovery.

Planning, inspection, help, query, and next-action selection are Read effects
with zero hidden writes. Every mutation receives its structural effect and
explicit lease. Stop only the dependent boundary for unavailable required
access, external authority, a secret boundary, or a destructive decision;
continue legal independent work.

## Interrupted recovery

After interruption, never trust the prior queue. Recompute the live candidate,
classify each lease as accepted, reviewable, stale, conflicted, abandoned, or
recoverable, preserve all unique uncommitted state, and resume only from a
current checkpoint. A worker summary or merge alone does not prove recovery.

## Output

Report the live graph, active and completed work packages, leases, reviews,
root decisions, integrations, candidate drift, preserved state, exact blockers,
next legal action, and highest honest candidate-only ceiling.

Parallel activity, worker results, receipts, tests, generated rows, or host goal
state do not prove orchestration, readiness, release, or completion.
