# ExecPlans, Worktrees, and Orchestration

## ExecPlans

Long-running or multi-lane work uses one self-contained living ExecPlan. A
future agent must be able to restart from it alone.

The plan includes purpose, observable outcomes, current progress, discoveries,
decisions, owned/shared paths, dependency graph, concrete steps, validation,
blockers, idempotence, recovery, retained artifacts, and claim ceiling.

Revise the plan as work proceeds. Do not leave stale state because chat later
corrected it.

## Isolation and ownership

Concurrent lanes use isolated branches, worktrees, state roots, artifact
roots, ports, caches, targets, scratch paths, credentials, and mutable runtime
state.

- Preserve user changes and never sweep unrelated files into a lane.
- Dirty worktrees and foreign edits are blockers or coordination points, not
  permission to widen scope.
- A lane writes only its exclusive paths and owns only its declared semantic
  decisions. Worktree isolation does not cure overlapping authority.
- All sibling lanes consume the same root-frozen base and shared interface.
  They do not merge, cherry-pick, copy, or inspect one another's unintegrated
  work.
- A shared-interface change cancels only declared consumers.
- Runtime output belongs in ignored lane-local state.
- Preserve durable evidence only while a current claim, irreproducible
  observation, custody handoff, or recovery need consumes it.
- Reuse one worktree across a lane's bounded repair loop. Close a clean merged
  worktree after its branch tip and required evidence are preserved.

Codex app-managed worktree tasks are preferred for substantial macro-lanes
that need visible independent continuation. This preference does not override
the active goal, phase order, ownership, authority, or effect restrictions.
Create or verify the branch ref before creating its worktree.

Use the lowest supported model and reasoning that fits the named lane risk.
Record model/reasoning only when exposed; otherwise use `unknown`.

## Orchestrator responsibilities

The root orchestrator owns the graph, frozen interface, workspaces,
dependencies, readiness, fan-in, shared wiring, proof joins, claim decisions,
and teardown. It does not implement lane-local behavior.

For every lane it can answer:

- owner, branch, worktree, base commit/tree, and current head/tree;
- exact owned, forbidden, and shared paths;
- exact shared-interface fields and dependencies consumed;
- local oracle, proof tier, budget, and stop condition;
- current disposition and freshness;
- minimal evidence and claim ceiling; and
- next action and teardown condition.

Lane acceptance is not root acceptance. Root verifies ancestry and ownership,
rejects peer merges and semantic conflicts, merges accepted lanes once in the
declared order, applies only root-owned wiring, runs integrated checks, freezes
one candidate, and closes or cancels every lane.

`PLANS.md` is stable law. The single active ExecPlan owns current lane state.
Add a registry, backlog, completion manifest, or receipt only when a current
cross-process consumer needs a machine contract the plan cannot safely supply.

## Lane return envelope

A lane returns:

- lane id, branch, worktree, base/head commits and trees;
- disposition: `accepted_candidate`, `no_change`, or `blocked`;
- exact changed owned paths and confirmation that forbidden/shared paths were
  not changed;
- consumed shared-interface fields and relevant dependencies;
- focused commands, exit codes, and material outcomes;
- required failure-path or boundary evidence;
- requested root-owned changes not made;
- residual risk and local claim ceiling; and
- clean/dirty state plus safe teardown disposition.

The envelope is a message, not a durable receipt by default.
