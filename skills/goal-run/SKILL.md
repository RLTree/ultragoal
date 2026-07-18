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
proof boundaries. Also require the current first-value event, operator outcome,
review and integration capacity, and any observed attention, coordination,
latency, or retained-artifact cost that changes the next decision. Record
model, mode, reasoning, and runtime configuration only when the runtime exposes
them directly.

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

## Select the next investment

Apply this order without writing a parallel queue or score ledger:

1. Integrate an accepted exact candidate.
2. Repair the rejected critical-path candidate at the shared invariant.
3. Unblock the nearest dependency-legal installed operator journey or first
   value event.
4. Investigate only when uncertainty blocks one of those actions.
5. Start another implementation stream only when it is substantial,
   dependency-independent, disjoint in authority and effects, and root review
   and integration capacity exists.

Prefer the coherent slice that delivers the most operator value with the least
avoidable human attention, coordination, rebuild, review, and retained-artifact
cost. This is a selection rule, not permission to bypass security, authority,
correctness, product quality, or a required claim boundary.

## Orchestrate

1. Build a dependency-closed graph and keep independent ready work active only
   while integration and review capacity can absorb it. Use single-lane
   execution for an ordered chain; use Ultra only for at least two substantial
   independent streams.
2. Retain shared manifests, dependencies, public commands, generated authority,
   migration, integration, claim decisions, release, and final reporting at the
   root.
3. Give each implementation worker one run-scoped lease disjoint in paths,
   semantic symbols, generated outputs, fixtures, scratch roots, and effects.
4. Freeze a material candidate, assign one risk-matched reviewer who did not
   implement it, and attack the complete named invariant, including applicable
   scope escape, stale identity, false passes, security, race, interruption,
   and recovery. A clean exhaustive pass closes that review loop.
5. Emit the existing `WorkerResult-v1` once after source acceptance. Workers
   may request root changes but never apply them or claim acceptance.
6. Integrate accepted work immediately; issue a corrective lease for material
   rework. Recompute every affected digest and claim ceiling.
7. Checkpoint the candidate, accepted sets, preserved state, open findings,
   blockers, and one exact next action after every material change or recovery.

Use one lean Goal, Success, Context, Constraints, Output, Verification contract
across execution routes. Define the representative acceptance check first, keep
the prompt stable while comparing routes, and choose the lowest sufficient
controllable model and reasoning. More model work, agents, checks, or durable
evidence count as improvements only when the result still meets the quality bar
and their added attention and coordination cost is justified.

Focused repair checks and reproducible diagnostics remain ephemeral. Run broad
strict proof only at the integration or claim boundary it can support, and
persist only the smallest current proof or recovery anchor that another process
or claim actually consumes.

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
