# ExecPlans

> PLANS.md is stable ExecPlan law, not the project plan ledger.
> Adapt it only for repo terminology and path conventions. Do not put active
> project status, worker or thread ids, phase progress, backlog items, receipt
> state, or completion claims here.
> Project-specific state belongs in `docs/exec-plans/active/*`,
> `LANE_REGISTRY.json`, `VERIFICATION_BACKLOG.json`,
> `COMPLETION_MANIFEST.json`, receipts, or `AMENDMENTS.jsonl`.

An ExecPlan is a self-contained living design document that a coding agent can follow to deliver a working behavior.

## Non-Negotiable Requirements

- The ExecPlan must be self-contained.
- A novice agent must be able to restart from the ExecPlan alone.
- The ExecPlan must produce demonstrably working behavior.
- Terms of art must be defined in plain language.
- Progress, discoveries, decisions, outcomes, validation, and recovery must stay current.

## Required Sections

- Purpose / Big Picture
- Progress
- Surprises & Discoveries
- Decision Log
- Outcomes & Retrospective
- Context and Orientation
- Plan of Work
- Concrete Steps
- Validation and Acceptance
- Idempotence and Recovery
- Artifacts and Notes
- Interfaces and Dependencies

## Lane Extension

For macro-lanes, also include:

- owner;
- owner thread type, usually a Codex app-managed worktree thread for
  long-running macro-lanes that need sidebar visibility, resumability, or user
  handoff;
- lane-owner model and reasoning when Codex exposes them, using the lowest
  supported reasoning level that fits the named risk and recording `unknown`
  rather than inferring unavailable metadata;
- launch prompt path or exact prompt text used to bind the lane agent to this
  ExecPlan;
- branch and worktree creation order, including the rule that branch refs are
  created or verified before an app worktree is requested from that branch;
- worktree path or creation receipt, and the command or `codex_app.create_thread`
  target used to create it;
- per-worktree environment setup, including `.codex-worktree/env.sh` or the
  repo's equivalent generated env file that agents must source before
  validation commands;
- state roots and artifact roots;
- port, cache, target, temp, scratch, and home isolation requirements;
- owned paths;
- forbidden/shared paths;
- dependencies;
- dependency blocker lifting rule: when upstream dependencies land and root
  verification passes, the orchestrator launches or resumes newly unblocked
  lane worktree threads without waiting for another user nudge;
- dependency landing and dependent-worktree advancement rules;
- claim ceiling;
- live beneficial end-to-end proof requirement;
- ready receipt path;
- review cadence and required reviewer personas when the lane is material;
- review model and reasoning: every material sign-off round uses the four
  merged canonical personas against the current validator, review-target,
  archive, registry, and claim-ceiling anchors, with runtime-supported
  configuration recorded only when exposed;
- parent-thread completion message contract;
- teardown condition, including what proves the branch tip is preserved, the
  worktree is clean, evidence has been captured, and the worktree can be closed.

## Orchestrator Responsibilities

An ExecPlan macro-lane must make orchestration state visible enough that a
future parent session can continue without guessing. The plan must name the
lane owner, branch, worktree, dependencies, proof anchors, blockers, and next
action. If any of those are unknown, the plan records the gap and the exact
probe required to resolve it.

The orchestrator is responsible for creating or verifying lane workspaces,
keeping dependency order honest, merging landed dependency lanes into the root
integration branch, advancing dependent worktrees only when it preserves their
scoped changes, and routing conflicts back to the lane owner when semantic
choices are required. The parent does not force, reset, stash, or repair a dirty
lane from outside the lane context unless the plan explicitly authorizes that
reconciliation.

Codex app worktree threads are the preferred lane owners for substantial
macro-lanes because they are app-visible and resumable. When creating one from
a branch starting state, create or verify the branch first, then create the
worktree thread. A failed worktree initialization caused by a missing branch is
an orchestration defect that must be recorded in `Surprises & Discoveries` and
fixed before the lane can proceed.

Future lane-owner threads should use the lowest supported reasoning level that
fits the lane risk. Do not copy reviewer reasoning onto implementation lane
owners. Increase lane-owner reasoning only when the ExecPlan names the risk,
scope, and expected payoff. Record model and reasoning only when exposed.

## Lane Ready Message

A lane agent does not tell the parent "done" without a ready package. The final
lane message to the parent includes:

- lane id, branch, worktree, and current commit;
- changed owned paths and confirmation that forbidden/shared paths were not
  modified;
- exact commands run with exit codes and artifact paths;
- ready receipt path, validator receipt path when relevant, and claim ceiling;
- current worktree status, including any preserved uncommitted paths;
- blockers, withheld claims, dependency changes, and next recommended parent
  action.

The parent then either steers repair, merges the lane, runs root verification,
updates the lane registry, tears down the worktree, or launches the next
newly-unblocked macro-lane.
